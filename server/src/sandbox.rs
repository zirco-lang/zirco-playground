use std::{process::Stdio, time::Duration};

use nix::libc;
use tokio::{io::AsyncReadExt, process::Command, time::timeout};
use tracing::debug;

use crate::models::{Job, JobResult};

const COMMON_PRLIMIT_ARGS: [&str; 4] = [
    "--as=536870912",    // 512 MB
    "--cpu=10",          // 10 seconds of CPU time
    "--fsize=104857600", // 100 MB file size
    "--",
];

const OUTPUT_LIMIT_BYTES: usize = 1024 * 100; // 100 KiB
const COMPILE_TIMEOUT: Duration = Duration::from_secs(10);
const EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);
const OUTPUT_LIMIT_WARNING: &str =
    "[warning] Process output exceeded 100 KiB; the process was SIGKILLed.\n";

async fn cleanup_work_dir(work_dir: &str) {
    let _ = tokio::fs::remove_dir_all(work_dir).await;
}

#[derive(Debug)]
struct CapturedStream {
    bytes: Vec<u8>,
    truncated: bool,
}

async fn capture_stream<R>(
    mut reader: R,
    pid: libc::pid_t,
    stream_name: &'static str,
) -> Result<CapturedStream, String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 512];
    let mut truncated = false;

    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|e| format!("Failed to read {stream_name}: {e}"))?;

        if read == 0 {
            break;
        }

        if bytes.len() < OUTPUT_LIMIT_BYTES {
            let remaining = OUTPUT_LIMIT_BYTES - bytes.len();
            let take = remaining.min(read);
            bytes.extend_from_slice(&buffer[..take]);
            if take < read || bytes.len() >= OUTPUT_LIMIT_BYTES {
                truncated = true;
            }
        } else {
            truncated = true;
        }

        if truncated {
            unsafe {
                let _ = libc::kill(pid, libc::SIGKILL);
            }
            break;
        }
    }

    Ok(CapturedStream { bytes, truncated })
}

fn prlimit_command(program: &str) -> Command {
    let mut command = Command::new("prlimit");
    command.args(COMMON_PRLIMIT_ARGS);
    command.arg(program);
    command
}

fn zrc_command(program: &str, source_path: &str) -> Command {
    let mut command = prlimit_command(program);
    command.args([
        "-I",
        "./zrc-nightly/include",
        "-I",
        "./zrc-nightly/libzr/include",
        "--forbid-unlisted-includes",
        source_path,
    ]);
    command
}

pub async fn sandboxed_execution(job: Job) -> Result<JobResult, String> {
    let work_dir = format!("./work/{}", job.id);
    let source_path = format!("./work/{}/main.zr", job.id);
    let obj_path = format!("./work/{}/main.o", job.id);
    let main_path = format!("./work/{}/main", job.id);
    tokio::fs::create_dir_all(work_dir.clone())
        .await
        .map_err(|e| format!("Failed to create work directory: {e}"))?;

    tokio::fs::write(&source_path, job.code)
        .await
        .map_err(|e| format!("Failed to write source file: {e}"))?;

    if job.task_type == crate::models::TaskType::Lint {
        // For linting, we invoke zircop instead of compiling and executing.

        debug!("Starting linting for job {}", job.id);

        let lint_result = timeout(
            COMPILE_TIMEOUT,
            zrc_command("./zrc-nightly/bin/zircop", &source_path).output(),
        )
        .await;

        let lint_result = match lint_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                // Clean up the work directory after execution
                cleanup_work_dir(&work_dir).await;
                return Err(format!("Failed to spawn linting process: {e}"));
            }
            Err(_) => {
                // Clean up the work directory after execution
                cleanup_work_dir(&work_dir).await;
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr: "Linting timed out after 10 seconds".to_string(),
                    exit_code: -1,
                });
            }
        };

        // Clean up the work directory after execution
        cleanup_work_dir(&work_dir).await;

        return Ok(JobResult {
            stdout: String::from_utf8_lossy(&lint_result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&lint_result.stderr).to_string(),
            exit_code: lint_result.status.code().unwrap_or(-1),
        });
    }

    debug!("Starting compilation for job {}", job.id);

    if job.task_type == crate::models::TaskType::Tast {
        // For TAST, we invoke zrc with --emit tast and return the output without linking or executing.

        let tast_result = timeout(COMPILE_TIMEOUT, {
            let mut command = zrc_command("./zrc-nightly/bin/zrc", &source_path);
            command.args(["--emit", "tast"]);
            command.output()
        })
        .await;

        let tast_result = match tast_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                // Clean up the work directory after execution
                cleanup_work_dir(&work_dir).await;
                return Err(format!("Failed to spawn TAST generation process: {e}"));
            }
            Err(_) => {
                // Clean up the work directory after execution
                cleanup_work_dir(&work_dir).await;
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr: "TAST generation timed out after 10 seconds".to_string(),
                    exit_code: -1,
                });
            }
        };

        // Clean up the work directory after execution
        cleanup_work_dir(&work_dir).await;

        return Ok(JobResult {
            stdout: String::from_utf8_lossy(&tast_result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&tast_result.stderr).to_string(),
            exit_code: tast_result.status.code().unwrap_or(-1),
        });
    } else if job.task_type == crate::models::TaskType::Llvm {
        // For LLVM IR, we invoke zrc with --emit llvm and return the output without linking or executing.

        let llvm_result = timeout(COMPILE_TIMEOUT, {
            let mut command = zrc_command("./zrc-nightly/bin/zrc", &source_path);
            command.args(["--emit", "llvm"]);
            command.output()
        })
        .await;

        let llvm_result = match llvm_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                // Clean up the work directory after execution
                cleanup_work_dir(&work_dir).await;
                return Err(format!("Failed to spawn LLVM IR generation process: {e}"));
            }
            Err(_) => {
                // Clean up the work directory after execution
                cleanup_work_dir(&work_dir).await;
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr: "LLVM IR generation timed out after 10 seconds".to_string(),
                    exit_code: -1,
                });
            }
        };

        // Clean up the work directory after execution
        cleanup_work_dir(&work_dir).await;

        return Ok(JobResult {
            stdout: String::from_utf8_lossy(&llvm_result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&llvm_result.stderr).to_string(),
            exit_code: llvm_result.status.code().unwrap_or(-1),
        });
    }

    let compile_result = timeout(COMPILE_TIMEOUT, {
        let mut command = zrc_command("./zrc-nightly/bin/zrc", &source_path);
        command.args(["--emit", "object", "-o", &obj_path]);
        command.stdout(Stdio::null());
        command.output()
    })
    .await;

    match compile_result {
        Ok(Ok(output)) => {
            if output.status.success() {
                debug!("Compilation succeeded for job {}", job.id);
                // compilation completed successfully, proceed to execution
            } else {
                debug!(
                    "Compilation failed for job {} with exit code {:?}",
                    job.id,
                    output.status.code()
                );
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                // Clean up the work directory after execution
                cleanup_work_dir(&work_dir).await;
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr,
                    exit_code: output.status.code().unwrap_or(-1),
                });
            }
        }
        Ok(Err(e)) => {
            // Clean up the work directory after execution
            cleanup_work_dir(&work_dir).await;
            return Err(format!("Failed to spawn compilation process: {e}"));
        }
        Err(_) => {
            // Clean up the work directory after execution
            cleanup_work_dir(&work_dir).await;
            return Ok(JobResult {
                stdout: "".to_string(),
                stderr: "Compilation timed out after 10 seconds".to_string(),
                exit_code: -1,
            });
        }
    }

    debug!("Starting linking for job {}", job.id);

    // Now run clang -lc -lzr -o main main.o
    let link_result = timeout(COMPILE_TIMEOUT, {
        let mut command = prlimit_command("clang");
        command.args([
            &obj_path,
            "-o",
            &main_path,
            "./zrc-nightly/libzr/lib/libzr.a",
            "-lc",
            "-static",
        ]);
        command.stdout(Stdio::null());
        command.output()
    })
    .await;

    match link_result {
        Ok(Ok(output)) => {
            if output.status.success() {
                debug!("Linking succeeded for job {}", job.id);
                // linking completed successfully, proceed to execution
            } else {
                // Clean up the work directory after execution
                cleanup_work_dir(&work_dir).await;
                debug!(
                    "Linking failed for job {} with exit code {:?}",
                    job.id,
                    output.status.code()
                );
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr,
                    exit_code: output.status.code().unwrap_or(-1),
                });
            }
        }
        Ok(Err(e)) => {
            // Clean up the work directory after execution
            cleanup_work_dir(&work_dir).await;
            return Err(format!("Failed to spawn linking process: {e}"));
        }
        Err(_) => {
            // Clean up the work directory after execution
            cleanup_work_dir(&work_dir).await;
            return Ok(JobResult {
                stdout: "".to_string(),
                stderr: "Linking timed out after 10 seconds".to_string(),
                exit_code: -1,
            });
        }
    }

    // Now we spawn the process within nsjail.
    // --chroot to its workdir
    // --time-limit 30
    // --rlimit_as 512M
    // --rlimit_cpu 30
    // --seccomp_file ./seccomp.bpf
    let normalized_work_dir = std::fs::canonicalize(&work_dir)
        .map_err(|e| format!("Failed to canonicalize work directory: {e}"))?
        .to_str()
        .ok_or_else(|| "Failed to convert work directory path to string".to_string())?
        .to_string();

    debug!("Starting execution for job {}", job.id);
    let mut exec_command = Command::new("nsjail");
    exec_command.args([
        "--quiet",
        "--bindmount",
        &format!("{}:/work", normalized_work_dir),
        "--time_limit",
        "30",
        "--rlimit_as",
        "536870912",
        "--rlimit_cpu",
        "30",
        "--rlimit_nofile",
        "20",
        "--seccomp_policy",
        "./seccomp.policy",
        "--user",
        "9999",
        "--group",
        "9999",
        "--",
        "/work/main",
    ]);
    exec_command.stdin(Stdio::null());
    exec_command.stdout(Stdio::piped());
    exec_command.stderr(Stdio::piped());

    let mut child = exec_command
        .spawn()
        .map_err(|e| format!("Failed to spawn execution process: {e}"))?;

    let pid = child
        .id()
        .map(|raw_pid| raw_pid as libc::pid_t)
        .ok_or_else(|| "Failed to get execution process id".to_string())?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture execution stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture execution stderr".to_string())?;

    let stdout_handle = tokio::spawn(capture_stream(stdout, pid, "stdout"));
    let stderr_handle = tokio::spawn(capture_stream(stderr, pid, "stderr"));

    let exec_status = match timeout(EXECUTION_TIMEOUT, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            unsafe {
                let _ = libc::kill(pid, libc::SIGKILL);
            }
            let _ = stdout_handle.await;
            let _ = stderr_handle.await;
            cleanup_work_dir(&work_dir).await;
            return Err(format!("Failed to wait for execution process: {e}"));
        }
        Err(_) => {
            unsafe {
                let _ = libc::kill(pid, libc::SIGKILL);
            }
            let _ = stdout_handle.await;
            let _ = stderr_handle.await;
            cleanup_work_dir(&work_dir).await;
            return Ok(JobResult {
                stdout: "".to_string(),
                stderr: "Execution timed out after 30 seconds".to_string(),
                exit_code: -1,
            });
        }
    };

    let stdout_capture = stdout_handle
        .await
        .map_err(|e| format!("Failed to join stdout reader: {e}"))??;
    let stderr_capture = stderr_handle
        .await
        .map_err(|e| format!("Failed to join stderr reader: {e}"))??;

    let stdout = String::from_utf8_lossy(&stdout_capture.bytes).to_string();
    let mut stderr = String::from_utf8_lossy(&stderr_capture.bytes).to_string();

    if stdout_capture.truncated || stderr_capture.truncated {
        stderr.push_str(OUTPUT_LIMIT_WARNING);
    }

    // Clean up the work directory after execution
    cleanup_work_dir(&work_dir).await;

    Ok(JobResult {
        stdout,
        stderr,
        exit_code: exec_status.code().unwrap_or(-1),
    })
}

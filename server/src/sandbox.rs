use std::{process::Stdio, time::Duration};

use tokio::{process::Command, time::timeout};
use tracing::debug;

use crate::models::{Job, JobResult};

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
            Duration::from_secs(10),
            Command::new("prlimit")
                .args([
                    "--as=536870912",    // 512 MB
                    "--cpu=10",          // 10 seconds of CPU time
                    "--fsize=104857600", // 100 MB file size
                    "--",
                    "./zrc-nightly/bin/zircop",
                    "-I",
                    "./zrc-nightly/include",
                    "-I",
                    "./zrc-nightly/libzr/include",
                    "--forbid-unlisted-includes",
                    &source_path,
                ])
                .output(),
        )
        .await;

        let lint_result = match lint_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                // Clean up the work directory after execution
                let _ = tokio::fs::remove_dir_all(work_dir).await;
                return Err(format!("Failed to spawn linting process: {e}"));
            }
            Err(_) => {
                // Clean up the work directory after execution
                let _ = tokio::fs::remove_dir_all(work_dir).await;
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr: "Linting timed out after 10 seconds".to_string(),
                    exit_code: -1,
                });
            }
        };

        // Clean up the work directory after execution
        let _ = tokio::fs::remove_dir_all(work_dir).await;

        return Ok(JobResult {
            stdout: String::from_utf8_lossy(&lint_result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&lint_result.stderr).to_string(),
            exit_code: lint_result.status.code().unwrap_or(-1),
        });
    }

    debug!("Starting compilation for job {}", job.id);

    if job.task_type == crate::models::TaskType::Tast {
        // For TAST, we invoke zrc with --emit tast and return the output without linking or executing.

        let tast_result = timeout(
            Duration::from_secs(10),
            Command::new("prlimit")
                .args([
                    "--as=536870912",    // 512 MB
                    "--cpu=10",          // 10 seconds of CPU time
                    "--fsize=104857600", // 100 MB file size
                    "--",
                    "./zrc-nightly/bin/zrc",
                    "-I",
                    "./zrc-nightly/include",
                    "-I",
                    "./zrc-nightly/libzr/include",
                    "--emit",
                    "tast",
                    "--forbid-unlisted-includes",
                    &source_path,
                ])
                .output(),
        )
        .await;

        let tast_result = match tast_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                // Clean up the work directory after execution
                let _ = tokio::fs::remove_dir_all(work_dir).await;
                return Err(format!("Failed to spawn TAST generation process: {e}"));
            }
            Err(_) => {
                // Clean up the work directory after execution
                let _ = tokio::fs::remove_dir_all(work_dir).await;
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr: "TAST generation timed out after 10 seconds".to_string(),
                    exit_code: -1,
                });
            }
        };

        // Clean up the work directory after execution
        let _ = tokio::fs::remove_dir_all(work_dir).await;

        return Ok(JobResult {
            stdout: String::from_utf8_lossy(&tast_result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&tast_result.stderr).to_string(),
            exit_code: tast_result.status.code().unwrap_or(-1),
        });
    } else if job.task_type == crate::models::TaskType::Llvm {
        // For LLVM IR, we invoke zrc with --emit llvm and return the output without linking or executing.

        let llvm_result = timeout(
            Duration::from_secs(10),
            Command::new("prlimit")
                .args([
                    "--as=536870912",    // 512 MB
                    "--cpu=10",          // 10 seconds of CPU time
                    "--fsize=104857600", // 100 MB file size
                    "--",
                    "./zrc-nightly/bin/zrc",
                    "-I",
                    "./zrc-nightly/include",
                    "-I",
                    "./zrc-nightly/libzr/include",
                    "--emit",
                    "llvm",
                    "--forbid-unlisted-includes",
                    &source_path,
                ])
                .output(),
        )
        .await;

        let llvm_result = match llvm_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                // Clean up the work directory after execution
                let _ = tokio::fs::remove_dir_all(work_dir).await;
                return Err(format!("Failed to spawn LLVM IR generation process: {e}"));
            }
            Err(_) => {
                // Clean up the work directory after execution
                let _ = tokio::fs::remove_dir_all(work_dir).await;
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr: "LLVM IR generation timed out after 10 seconds".to_string(),
                    exit_code: -1,
                });
            }
        };

        // Clean up the work directory after execution
        let _ = tokio::fs::remove_dir_all(work_dir).await;

        return Ok(JobResult {
            stdout: String::from_utf8_lossy(&llvm_result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&llvm_result.stderr).to_string(),
            exit_code: llvm_result.status.code().unwrap_or(-1),
        });
    }

    let compile_result = timeout(
        Duration::from_secs(10),
        Command::new("prlimit")
            .args([
                "--as=536870912",    // 512 MB
                "--cpu=10",          // 10 seconds of CPU time
                "--fsize=104857600", // 100 MB file size
                "--",
                "./zrc-nightly/bin/zrc",
                "-I",
                "./zrc-nightly/include",
                "-I",
                "./zrc-nightly/libzr/include",
                "--emit",
                "object",
                "-o",
                &obj_path,
                "--forbid-unlisted-includes",
                &source_path,
            ])
            .stdout(Stdio::null())
            .output(),
    )
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
                let _ = tokio::fs::remove_dir_all(work_dir).await;
                return Ok(JobResult {
                    stdout: "".to_string(),
                    stderr,
                    exit_code: output.status.code().unwrap_or(-1),
                });
            }
        }
        Ok(Err(e)) => {
            // Clean up the work directory after execution
            let _ = tokio::fs::remove_dir_all(work_dir).await;
            return Err(format!("Failed to spawn compilation process: {e}"));
        }
        Err(_) => {
            // Clean up the work directory after execution
            let _ = tokio::fs::remove_dir_all(work_dir).await;
            return Ok(JobResult {
                stdout: "".to_string(),
                stderr: "Compilation timed out after 10 seconds".to_string(),
                exit_code: -1,
            });
        }
    }

    debug!("Starting linking for job {}", job.id);

    // Now run clang -lc -lzr -o main main.o
    let link_result = timeout(
        Duration::from_secs(10),
        Command::new("prlimit")
            .args([
                "--as=536870912",    // 512 MB
                "--cpu=10",          // 10 seconds of CPU time
                "--fsize=104857600", // 100 MB file size
                "--",
                "clang",
                &obj_path,
                "-o",
                &main_path,
                "./zrc-nightly/libzr/lib/libzr.a",
                "-lc",
                "-static",
            ])
            .stdout(Stdio::null())
            .output(),
    )
    .await;

    match link_result {
        Ok(Ok(output)) => {
            if output.status.success() {
                debug!("Linking succeeded for job {}", job.id);
                // linking completed successfully, proceed to execution
            } else {
                // Clean up the work directory after execution
                let _ = tokio::fs::remove_dir_all(work_dir).await;
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
            let _ = tokio::fs::remove_dir_all(work_dir).await;
            return Err(format!("Failed to spawn linking process: {e}"));
        }
        Err(_) => {
            // Clean up the work directory after execution
            let _ = tokio::fs::remove_dir_all(work_dir).await;
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
    let exec_result = timeout(
        Duration::from_secs(30),
        Command::new("nsjail")
            .args([
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
            ])
            .output(),
    )
    .await;

    let exec_result = match exec_result {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            // Clean up the work directory after execution
            let _ = tokio::fs::remove_dir_all(work_dir).await;
            return Err(format!("Failed to spawn execution process: {e}"));
        }
        Err(_) => {
            // Clean up the work directory after execution
            let _ = tokio::fs::remove_dir_all(work_dir).await;
            return Ok(JobResult {
                stdout: "".to_string(),
                stderr: "Execution timed out after 30 seconds".to_string(),
                exit_code: -1,
            });
        }
    };

    // Clean up the work directory after execution
    let _ = tokio::fs::remove_dir_all(work_dir).await;

    Ok(JobResult {
        stdout: String::from_utf8_lossy(&exec_result.stdout).to_string(),
        stderr: String::from_utf8_lossy(&exec_result.stderr).to_string(),
        exit_code: exec_result.status.code().unwrap_or(-1),
    })
}

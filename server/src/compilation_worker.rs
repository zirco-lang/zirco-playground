use std::time::Duration;

use crate::sandbox;
use tracing::{debug, error, info};

use crate::models::{Job, JobResult, Results};

pub async fn worker(i: usize, rx: async_channel::Receiver<Job>, results: Results) {
    info!("Worker {i} started");

    loop {
        let job = match rx.recv().await {
            Ok(job) => job,
            Err(e) => {
                error!("Worker {i} shutting down: {e}");
                continue;
            }
        };

        debug!("Worker {i} received job: {job:?}");

        let id = job.id;
        let result = match sandbox::sandboxed_execution(job).await {
            Ok(res) => res,
            Err(e) => {
                error!("Worker {i} failed to execute job {}: {e}", id);
                JobResult {
                    stdout: "".to_string(),
                    stderr: format!("Fatal execution error: {e}"),
                    exit_code: -1,
                }
            }
        };

        results.lock().await.insert(id, result);

        debug!("Worker {i} completed job {id}");

        // Clean up job from results after some time to prevent memory bloat
        let results_clone = results.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_mins(5)).await;
            debug!("Worker {i} cleaning up results for job {id}");
            results_clone.lock().await.remove(&id);
        });
    }
}

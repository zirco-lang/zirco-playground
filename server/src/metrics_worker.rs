use tracing::{debug, error, info};

use crate::models::{Job, Results};

pub async fn main(rx: async_channel::Receiver<Job>, _results: Results) {
    info!("Metrics worker started");
    loop {
        // We write the metrics (number of pending jobs) to the file ./metrics.json every 30 seconds

        let pending_jobs = rx.len();
        let metrics = serde_json::json!({
            "pending_jobs": pending_jobs,
        });
        debug!("Metrics file written: {pending_jobs} pending jobs");

        if let Err(e) = tokio::fs::write(
            "./metrics.json",
            serde_json::to_string_pretty(&metrics).unwrap(),
        )
        .await
        {
            error!("Failed to write metrics file: {e}");
        }

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

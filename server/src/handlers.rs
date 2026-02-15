use std::{convert::Infallible, time::Duration};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use futures::{Stream, stream};
use tracing::debug;
use uuid::Uuid;

use crate::models::{AppState, ExecuteRequest, ExecuteResponse, Job, JobResult};

pub async fn execute_code(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, StatusCode> {
    let job_id = uuid::Uuid::new_v4();
    let job = Job {
        id: job_id,
        task_type: req.task,
        code: req.code,
    };

    debug!("Sending new job {job_id} to work queue");

    state
        .work_queue
        .send(job)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ExecuteResponse { job_id }))
}

enum StreamState {
    Pending(u32),
    Done,
}
pub async fn stream_results(
    Path(job_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let results = state.results.clone();

    let stream = stream::unfold(StreamState::Pending(0), move |count| {
        let results = results.clone();
        async move {
            match count {
                StreamState::Done => None,
                StreamState::Pending(count) => {
                    // Timeout after 60 seconds
                    // We check every 500ms
                    if count >= 120 {
                        let event = Event::default()
                            .event("timeout")
                            .json_data(serde_json::json!({
                                "error": "Timed out waiting for results."
                            }))
                            .ok()?;

                        return Some((Ok(event), StreamState::Done));
                    }

                    // Check for result
                    let result = results.lock().await.get(&job_id).cloned();

                    if let Some(result) = result {
                        // Send complete event and end stream
                        let event = Event::default().event("complete").json_data(&result).ok()?;
                        return Some((Ok(event), StreamState::Done));
                    }

                    // Still pending
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let event = Event::default().event("pending").data("running");

                    Some((Ok(event), StreamState::Pending(count + 1)))
                }
            }
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive"),
    )
}

pub async fn get_results(
    Path(job_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<JobResult>, StatusCode> {
    state
        .results
        .lock()
        .await
        .get(&job_id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)
        .map(Json)
}

// memoized version of getting the version from the zrc binary
pub async fn get_version() -> Json<serde_json::Value> {
    use once_cell::sync::OnceCell;
    static VERSION: OnceCell<String> = OnceCell::new();

    let version = VERSION.get_or_init(|| {
        let output = std::process::Command::new("./zrc-nightly/bin/zrc")
            .arg("--version")
            .output()
            .expect("Failed to execute zrc binary");

        String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string()
            .replace("zrc_cli", "Zirco")
    });

    Json(serde_json::json!({ "version": version }))
}

use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Execute,
    Lint,
    Tast,
    Llvm,
}

#[derive(Debug)]
pub struct Job {
    pub id: Uuid,
    pub task_type: TaskType,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub type Results = Arc<tokio::sync::Mutex<HashMap<Uuid, JobResult>>>;

#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    pub task: TaskType,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    #[serde(rename = "jobId")]
    pub job_id: Uuid,
}

/////

#[derive(Clone)]
pub struct AppState {
    pub work_queue: async_channel::Sender<Job>,
    pub results: Results,
}

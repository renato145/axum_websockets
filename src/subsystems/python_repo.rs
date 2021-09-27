use super::{Subsystem, WebsocketSystem};
use crate::error::error_chain_fmt;
use anyhow::Context;
use glob::glob;
use serde::Deserialize;
use std::path::Path;

#[derive(thiserror::Error)]
pub enum PythonRepoError {
    #[error("Invalid path: {0:?}")]
    InvalidPath(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PythonRepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

pub struct PythonRepoSystem;

#[async_trait::async_trait]
impl Subsystem for PythonRepoSystem {
    type Error = PythonRepoError;
    type Task = Task;

    fn system(&self) -> WebsocketSystem {
        WebsocketSystem::PythonRepo
    }

    async fn handle_message(
        &self,
        task: Self::Task,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, Self::Error> {
        match task {
            Task::GetFiles => get_files(payload),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    GetFiles,
}

#[tracing::instrument(name = "GetFiles task")]
fn get_files(payload: serde_json::Value) -> Result<serde_json::Value, PythonRepoError> {
    let path = payload.as_str().unwrap_or("");
    if !Path::new(path).exists() {
        return Err(PythonRepoError::InvalidPath(path.into()));
    }

    let files = glob(&format!("{}/**/*.py", path))
        .context("Failed to perform glob on path.")?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let result =
        serde_json::to_value(files).context("Failed to convert message to JSON format.")?;
    Ok(result)
}

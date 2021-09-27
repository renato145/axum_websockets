use super::{Subsystem, WebsocketSystem};
use crate::error::error_chain_fmt;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use systemstat::Platform;

#[derive(thiserror::Error)]
pub enum PcUsageError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PcUsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

pub struct PcUsageSystem;

#[async_trait::async_trait]
impl Subsystem for PcUsageSystem {
    type Error = PcUsageError;
    type Task = Task;

    fn system(&self) -> WebsocketSystem {
        WebsocketSystem::PcUsage
    }

    #[tracing::instrument(name = "Handling PcUsage message", skip(self))]
    async fn handle_message(
        &self,
        task: Self::Task,
        _payload: serde_json::Value,
    ) -> Result<serde_json::Value, Self::Error> {
        match task {
            Task::CpuLoad => get_cpu_load().await,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    CpuLoad,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuLoadResult {
    pub user: f32,
    pub system: f32,
}

#[tracing::instrument(name = "Handle task GetCpuLoad")]
async fn get_cpu_load() -> Result<serde_json::Value, PcUsageError> {
    let sys = systemstat::System::new();
    let cpu = sys
        .cpu_load()
        .context("Failed to initialize cpu load reader.")?;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let cpu_load = cpu
        .done()
        .context("Failed to read cpu load.")?
        .iter()
        .map(|cpu_load| CpuLoadResult {
            user: cpu_load.user,
            system: cpu_load.system,
        })
        .collect::<Vec<_>>();

    let result = serde_json::to_value(cpu_load).context("Failed to serialize cpu load result.")?;
    Ok(result)
}

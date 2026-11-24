use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::deployments::{DeploymentInfo, DeploymentOrigin, InstallProgress, InstalledDeployment, Reporter};
use crate::error::AppResult;
use crate::state::AppState;

const PROGRESS_EVENT: &str = "deployment-progress";

#[tauri::command]
pub fn list_deployments(state: State<'_, AppState>) -> AppResult<Vec<InstalledDeployment>> {
    state.deployments.list()
}

#[tauri::command]
pub async fn validate_deployment(state: State<'_, AppState>, version: String) -> AppResult<DeploymentInfo> {
    let host = state.settings.get().roblox.deployment_host;
    state.deployments.validate(&version, host).await
}

#[tauri::command]
pub async fn install_deployment(
    app: AppHandle,
    state: State<'_, AppState>,
    version: String,
) -> AppResult<InstalledDeployment> {
    let host = state.settings.get().roblox.deployment_host;
    let report: Reporter = Arc::new(move |progress: InstallProgress| {
        if let Err(error) = app.emit(PROGRESS_EVENT, progress) {
            log::warn!("could not send install progress: {error}");
        }
    });
    state.deployments.install(&version, host, report).await
}

#[tauri::command]
pub fn cancel_deployment_install(state: State<'_, AppState>) {
    state.deployments.cancel_install();
}

#[tauri::command]
pub fn remove_deployment(state: State<'_, AppState>, version: String) -> AppResult<()> {
    state.deployments.remove(&version)
}

#[tauri::command]
pub async fn launch_deployment(
    state: State<'_, AppState>,
    version: String,
    origin: DeploymentOrigin,
) -> AppResult<()> {
    // Signature checks hash the whole executable, so keep them off the main thread.
    let manager = state.deployments.clone();
    let flags = state.fflags.clone();
    tauri::async_runtime::spawn_blocking(move || manager.launch(&version, origin, |install| flags.apply_to(install)))
        .await?
}

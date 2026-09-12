use ordinconn_app::AppRuntime;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub fn forward_runtime_events(runtime: Arc<AppRuntime>, app: AppHandle) {
    let mut events = runtime.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => {
                    let _ = app.emit("ordinconn://runtime-event", event);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

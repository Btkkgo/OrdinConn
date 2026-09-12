mod commands;
mod credential_store;
mod events;
mod state;

use std::sync::Arc;
use tauri::{Manager, RunEvent};

pub fn run() {
    let builder = tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let runtime = tauri::async_runtime::block_on(ordinconn_app::AppRuntime::initialize(
                &data_dir.join("ordinconn-v0-1.sqlite3"),
            ))?;
            tauri::async_runtime::block_on(runtime.seed_demo_data())?;
            events::forward_runtime_events(Arc::clone(&runtime), app.handle().clone());
            app.manage(state::AppState::new(
                runtime,
                Arc::new(credential_store::SystemCredentialStore),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::start_agent_turn,
            commands::get_thread_items,
            commands::cancel_agent_turn,
            commands::create_report,
            commands::create_trade_proposal,
            commands::request_approval,
            commands::approve_and_execute_paper,
            commands::save_model_provider,
            commands::test_model_provider,
        ]);
    let app = builder
        .build(tauri::generate_context!())
        .expect("failed to build OrdinConn");
    app.run(|app_handle, event| {
        if matches!(event, RunEvent::ExitRequested { .. })
            && let Some(state) = app_handle.try_state::<state::AppState>()
        {
            let runtime = Arc::clone(&state.runtime);
            let _ = tauri::async_runtime::block_on(async move { runtime.shutdown().await });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::credential_store::{CredentialStore, MemoryCredentialStore};
    use super::state::redact_error;

    #[test]
    fn credential_store_round_trip_keeps_secret_out_of_debug_output() {
        let store = MemoryCredentialStore::default();
        store.set("provider-1", "sk-secret-value").unwrap();
        assert_eq!(
            store.get("provider-1").unwrap().as_deref(),
            Some("sk-secret-value")
        );
        assert!(!format!("{store:?}").contains("sk-secret-value"));
    }

    #[test]
    fn ipc_errors_redact_sensitive_values() {
        let message = redact_error("request failed with api key sk-secret-value and token abc123");
        assert!(!message.contains("sk-secret-value"));
        assert!(!message.contains("abc123"));
        assert!(message.contains("[REDACTED]"));
    }
}

//! Explicit production-provider / real-AVD acceptance; never substitutes planner decisions.
use crate::{
    credential_store::{CredentialStore, SystemCredentialStore},
    mobile::MobileHost,
    mobile_executor::*,
};
use mobile_runtime::execution::*;
use ordinconn_app::{AppRuntime, mobile_executor::MobileExecutorRuntime};
use serde_json::json;
use sqlx::Row;
use std::{path::PathBuf, sync::Arc, time::Duration};

#[test]
#[ignore = "requires explicit owner authorization, saved native credential, and dedicated real AVD"]
fn final_real_autonomous_acceptance() {
    assert_eq!(std::env::var("ORDINCONN_FINAL_M3_GATE").as_deref(), Ok("1"));
    let _lock = real_avd_gate_lock().expect("exclusive AVD gate");
    let host = Arc::new(MobileHost::discover());
    host.start_avd(None, "OrdinConn_M1_5", Duration::from_secs(120))
        .expect("dedicated AVD");
    let diagnostics = host.environment_diagnostics(None);
    assert_eq!(diagnostics.online_devices.len(), 1);
    let name = std::process::Command::new(diagnostics.adb_path.as_ref().unwrap())
        .args([
            "-s",
            &diagnostics.online_devices[0].id,
            "emu",
            "avd",
            "name",
        ])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&name.stdout)
            .lines()
            .any(|s| s.trim() == "OrdinConn_M1_5")
    );
    let database =
        PathBuf::from(std::env::var("ORDINCONN_FINAL_M3_DB").expect("production database path"));
    let evidence = PathBuf::from(
        std::env::var("ORDINCONN_FINAL_M3_EVIDENCE").expect("private evidence directory"),
    );
    assert!(database.is_file() && evidence.is_dir());
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let app = AppRuntime::initialize(&database).await.unwrap();
        let provider_rows = sqlx::query("SELECT id,provider_type,default_model,base_url,credential_ref,enabled FROM model_providers").fetch_all(app.pool()).await.unwrap();
        assert_eq!(provider_rows.len(), 1);
        let provider = &provider_rows[0];
        let provider_id: String = provider.get("id");
        assert_eq!(provider.get::<String,_>("provider_type"), "openai_compatible_chat");
        assert_eq!(provider.get::<String,_>("default_model"), "gemini-3.6-flash");
        assert_eq!(provider.get::<String,_>("base_url"), "https://generativelanguage.googleapis.com/v1beta/openai/");
        assert_eq!(provider.get::<i64,_>("enabled"), 1);
        assert_eq!(provider.get::<String,_>("credential_ref"), format!("keyring:{provider_id}"));
        let settings = app.mobile_workspace_data().await.unwrap().settings;
        assert_eq!(settings.allowed_apps, vec!["com.android.settings"]);
        let executor = Arc::new(DesktopMobileExecutorRuntime::new(host.clone(), settings.android_sdk, settings.allowed_apps));
        let initial = executor.observe().await.expect("real initial observation");
        assert_eq!(initial.snapshot.package_name, "com.android.settings");
        assert_eq!(initial.snapshot.package_name,"com.android.settings");
        assert_ne!(initial.snapshot.activity,"com.android.settings.homepage.SettingsHomepageActivity");
        assert!(initial.snapshot.elements.iter().any(|e|e.text.as_deref()==Some("Internet") || e.content_description.as_deref()==Some("Internet")));
        assert!(!initial.snapshot.elements.iter().any(|e|e.text.as_deref()==Some("SIMs")),"start inside Internet subpage, not its parent");
        println!("INITIAL observation={} activity={}",initial.observation.id,initial.snapshot.activity);
        app.record_mobile_capture(&initial).await.unwrap();
        let native = keyring::Entry::new("ai.ordinconn.desktop.model-provider", &provider_id).unwrap();
        assert!(!native.get_credential().is::<keyring::mock::MockCredential>());
        // Resolve before observation-sensitive planning so an OS consent prompt cannot stale a Step.
        assert!(SystemCredentialStore.get(&provider_id).ok().flatten().is_some_and(|s| !s.is_empty()));
        println!("Credential=KEYCHAIN_RESOLVED provider={provider_id} model=gemini-3.6-flash");
        let repo = app.mobile_goal_repository();
        let goal = repo.create_goal(
            "Return from this Internet settings page to the main Android Settings homepage without changing any setting.",
            MobileGoalBudget { max_runtime_ms: 300_000, ..Default::default() },
        ).await.unwrap();
        repo.set_completion_target(&goal.id, &MobileCompletionTarget::PageEquals {
            package: "com.android.settings".into(), activity: "com.android.settings.homepage.SettingsHomepageActivity".into(),
            visible_text:vec!["Search Settings".into(),"Network & internet".into(),"Connected devices".into()],
        }).await.unwrap();
        println!("FINAL_GOAL_ID={}", goal.id.as_str());
        let result = app.run_mobile_goal(&goal.id, executor.clone(), |id| {
            SystemCredentialStore.get(id).map_err(|_| MobileGoalError::new(MobileGoalErrorCode::ModelError).into())
        }).await;
        let saved = repo.get_goal(&goal.id).await.unwrap();
        let mut steps = Vec::new();
        for plan in repo.list_plans_for_goal(&goal.id).await.unwrap() {
            for step in repo.list_steps(&plan.id).await.unwrap() {
                let verification = repo.get_step_result(&step.id).await.unwrap();
                steps.push(json!({"planId":plan.id,"revision":plan.revision,"stepId":step.id,
                    "type":step.step_type,"status":step.status,"risk":step.risk,"expected":step.expected_result,
                    "before":step.observation_before_id,"after":step.observation_after_id,"receipt":step.action_id,
                    "verification":verification}));
            }
        }
        let decisions: Vec<String> = sqlx::query_scalar("SELECT json_object('id',id,'observationId',observation_id,'providerId',model_provider_id,'model',model_name,'stepId',step_id,'planId',plan_id) FROM mobile_planner_decisions WHERE goal_id=? ORDER BY created_at,id")
            .bind(goal.id.as_str()).fetch_all(app.pool()).await.unwrap();
        let decisions:Vec<serde_json::Value>=decisions.iter().map(|s|serde_json::from_str(s).unwrap()).collect();
        let calls:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_model_calls WHERE goal_id=?").bind(goal.id.as_str()).fetch_one(app.pool()).await.unwrap();
        let commands:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps s JOIN mobile_plans p ON p.id=s.plan_id JOIN mobile_action_receipts r ON r.id=s.action_id WHERE p.goal_id=? AND r.status='executed' AND json_extract(r.domain_json,'$.commandSent')=1")
            .bind(goal.id.as_str()).fetch_one(app.pool()).await.unwrap();
        let mut report=json!({"independentCompletion":false,"providerType":"openai_compatible_chat","goalId":goal.id,"status":saved.status,"error":saved.last_error_code,"modelCalls":calls,"realActions":commands,"decisions":decisions,"steps":steps});
        std::fs::write(evidence.join(format!("{}.json",goal.id.as_str())),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        println!("FINAL_GATE_REPORT={report}");
        if result.is_ok() && saved.status==MobileGoalStatus::Completed {
            let fresh=executor.observe().await.expect("independent final observation");
            let labels:Vec<&str>=fresh.snapshot.elements.iter().filter_map(|e|e.text.as_deref()).collect();
            println!("INDEPENDENT_OBSERVATION id={} activity={} labels={:?}",fresh.observation.id,fresh.snapshot.activity,labels);
            assert_eq!(fresh.snapshot.activity,"com.android.settings.homepage.SettingsHomepageActivity");
            assert!(labels.contains(&"Search Settings") && labels.contains(&"Network & internet") && labels.contains(&"Connected devices"),"independent Settings homepage verification");
            assert!(calls>=2 && commands>=2,"at least two genuine model decisions and real actions");
            assert!(decisions.len()>=2);
            for pair in decisions.windows(2) {
                assert_ne!(pair[0]["observationId"], pair[1]["observationId"], "no stale decision observation");
            }
            assert!(steps.iter().filter(|s|s["status"]=="VERIFIED").count()>=2);
            let verified:Vec<_>=steps.iter().filter(|s|s["status"]=="VERIFIED").collect();
            for pair in verified.windows(2) {
                let next=decisions.iter().find(|d|d["stepId"]==pair[1]["stepId"]).expect("real decision owns step");
                assert_eq!(next["observationId"],pair[0]["after"],"continuation uses preceding fresh post observation");
            }
            report["independentCompletion"]=json!(true);
            report["independentObservationId"]=json!(fresh.observation.id);
            report["semanticTarget"]=json!(["Search Settings","Network & internet","Connected devices"]);
            std::fs::write(evidence.join(format!("{}.json",goal.id.as_str())),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
            println!("FINAL_REAL_AUTONOMOUS_GATE=PASS independent_completion=PASS");
        } else {
            println!("FINAL_REAL_AUTONOMOUS_GATE=FAIL status={:?}",saved.status);
        }
        host.stop_session();
        app.shutdown().await.unwrap();
        assert!(result.is_ok() && saved.status==MobileGoalStatus::Completed, "real autonomous gate did not complete; durable evidence retained");
    });
}

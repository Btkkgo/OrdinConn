//! Opt-in, exclusive real device acceptance. Plans are fixtures; execution is production code.
use crate::{mobile::MobileHost, mobile_executor::*};
use mobile_runtime::{execution::*, *};
use ordinconn_app::{
    AppRuntime, MobileRuntimeSettings,
    mobile_executor::{MobileExecutorRuntime, MobileStepExecutionOutcome},
};
use std::{process::Command, sync::Arc, time::Duration};

async fn test_plan(
    app: &Arc<AppRuntime>,
    capture: &MobileCapture,
    kind: MobileStepType,
    expected: ExpectedStepResult,
    target: Option<String>,
    text: Option<String>,
) -> (MobileGoal, MobilePlanStep) {
    app.record_mobile_capture(capture).await.unwrap();
    let repo = app.mobile_goal_repository();
    let objective = if kind == MobileStepType::ScrollDown {
        "Scroll down the current Settings page once"
    } else {
        "Bounded public Settings acceptance step"
    };
    let goal = repo
        .create_goal(objective, MobileGoalBudget::default())
        .await
        .unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Planning, None)
        .await
        .unwrap();
    let plan = repo
        .create_plan(
            &goal.id,
            1,
            objective,
            vec![NewMobileStep {
                sequence: 1,
                step_type: kind,
                reason: "Public Settings device gate".into(),
                risk: if matches!(
                    kind,
                    MobileStepType::Extract | MobileStepType::Observe | MobileStepType::Wait
                ) {
                    MobileStepRisk::ReadOnly
                } else {
                    MobileStepRisk::Reversible
                },
                target_ref: target,
                input_text: text,
                expected_result: Some(expected),
                wait_ms: None,
                extraction_intent: if kind == MobileStepType::Extract {
                    Some("Visible public Settings labels".into())
                } else {
                    None
                },
            }],
        )
        .await
        .unwrap();
    repo.activate_plan(&plan.id).await.unwrap();
    let step = repo.list_steps(&plan.id).await.unwrap().remove(0);
    repo.attach_step_observations(&step.id, Some(capture.observation.id.clone()), None)
        .await
        .unwrap();
    (goal, step)
}
fn label_target(c: &MobileCapture, label: &str) -> Option<String> {
    let label = c
        .snapshot
        .elements
        .iter()
        .find(|e| e.text.as_deref() == Some(label))?;
    let x = label.bounds.x + label.bounds.width / 2;
    let y = label.bounds.y + label.bounds.height / 2;
    c.snapshot
        .elements
        .iter()
        .filter(|e| {
            e.enabled
                && e.clickable
                && e.bounds.x <= x
                && x < e.bounds.x + e.bounds.width
                && e.bounds.y <= y
                && y < e.bounds.y + e.bounds.height
        })
        .min_by_key(|e| e.bounds.width * e.bounds.height)
        .map(|e| e.element_ref.clone())
}
async fn assert_trace(app: &AppRuntime, out: &MobileStepExecutionOutcome, mutation: bool) {
    assert!(
        out.verified,
        "step error: {:?}; receipt: {:?}",
        out.error,
        app.mobile_workspace_data()
            .await
            .unwrap()
            .latest_action_receipt
    );
    assert_eq!(out.status, MobileStepStatus::Verified);
    let result = app
        .mobile_goal_repository()
        .get_step_result(&out.step_id)
        .await
        .unwrap()
        .unwrap();
    assert!(result.verified);
    assert_eq!(result.observation_before_id, out.observation_before_id);
    assert_eq!(result.observation_after_id, out.observation_after_id);
    if mutation {
        assert!(out.action_receipt_id.is_some());
        assert_ne!(out.observation_before_id, out.observation_after_id);
    }
}
#[test]
fn real_phase4_executor_gate_is_explicitly_gated() {
    if std::env::var("ORDINCONN_MOBILE_M3_EXECUTOR_SMOKE").as_deref() != Ok("1") {
        return;
    }
    let _avd_lock = real_avd_gate_lock().expect("exclusive real AVD gate");
    let host = Arc::new(MobileHost::discover());
    let mut diagnostics = host.environment_diagnostics(None);
    if diagnostics.online_devices.is_empty() {
        assert!(
            diagnostics
                .available_avds
                .iter()
                .any(|a| a.name == "OrdinConn_M1_5")
        );
        host.start_avd(None, "OrdinConn_M1_5", Duration::from_secs(120))
            .expect("dedicated AVD startup");
        diagnostics = host.environment_diagnostics(None);
    }
    assert_eq!(diagnostics.online_devices.len(), 1);
    let device = &diagnostics.online_devices[0].id;
    let adb = diagnostics.adb_path.as_deref().unwrap();
    let avd = Command::new(adb)
        .args(["-s", device, "emu", "avd", "name"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&avd.stdout)
            .lines()
            .any(|s| s.trim() == "OrdinConn_M1_5")
    );
    // Test preparation only: navigate to a public Settings page, without changing a setting.
    assert!(
        Command::new(adb)
            .args([
                "-s",
                device,
                "shell",
                "am",
                "start",
                "-W",
                "--activity-clear-top",
                "-a",
                "android.settings.DISPLAY_SETTINGS"
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    let ready_deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let focus = Command::new(adb)
            .args(["-s", device, "shell", "dumpsys", "window"])
            .output()
            .unwrap();
        if String::from_utf8_lossy(&focus.stdout).lines().any(|line| {
            line.contains("mCurrentFocus=Window") && line.contains("com.android.settings/")
        }) {
            break;
        }
        assert!(
            std::time::Instant::now() < ready_deadline,
            "test Settings surface never acquired focus"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    let allowed = vec![
        "com.android.settings".into(),
        "com.google.android.settings.intelligence".into(),
    ];
    let directory = tempfile::tempdir().unwrap();
    let executor = tokio::runtime::Runtime::new().unwrap();
    executor.block_on(async {
        let app=AppRuntime::initialize(&directory.path().join("m3-real-executor.sqlite3")).await.unwrap();
        app.save_mobile_settings(&MobileRuntimeSettings{allowed_apps:allowed.clone(),..Default::default()}).await.unwrap();
        let runtime=Arc::new(DesktopMobileExecutorRuntime::new(host.clone(),None,allowed.clone()));
        let initial=host.observe_with_sdk(None,&allowed).expect("real Display capture");
        assert!(mobile_runtime::executor::valid_capture(&initial),"Settings capture rejected: {:?}",initial.snapshot.sensitive_state);
        // BACK: enter a safe child page first and verify exact return activity.
        if let Some(target)=label_target(&initial,"Display size and text") {
            let (goal,_)=test_plan(&app,&initial,MobileStepType::TapElement,ExpectedStepResult::ActivityChanged,Some(target),None).await;
            let tap=app.execute_mobile_goal_step(&goal.id,runtime.clone()).await.unwrap();assert_trace(&app,&tap,true).await;
            let child=host.observe_with_sdk(None,&allowed).unwrap();
            let (goal,_)=test_plan(&app,&child,MobileStepType::Back,ExpectedStepResult::ActivityEquals {
                package:initial.snapshot.package_name.clone(),activity:initial.snapshot.activity.clone()},None,None).await;
            let back=app.execute_mobile_goal_step(&goal.id,runtime.clone()).await.unwrap();assert_trace(&app,&back,true).await;
            assert_eq!(host.current_capture().unwrap().snapshot.activity,initial.snapshot.activity);
            println!("M3_REAL_BACK=PASS typed=ACTIVITY_EQUALS");
        } else { println!("M3_REAL_BACK=NOT_RUN safe_child_page_unavailable"); }
        let before=host.observe_with_sdk(None,&allowed).unwrap();
        let (goal,step)=test_plan(&app,&before,MobileStepType::ScrollDown,ExpectedStepResult::UiChanged,None,None).await;
        app.mobile_goal_repository().set_mobile_completion_criterion(&goal.id,&step.id).await.unwrap();
        let scroll=app.execute_mobile_goal_step(&goal.id,runtime.clone()).await.unwrap();assert_trace(&app,&scroll,true).await;
        let after=host.current_capture().unwrap();assert!(!mobile_runtime::executor::same_app_ui(&before.snapshot,&after.snapshot));
        assert_eq!(app.mobile_goal_repository().get_goal(&goal.id).await.unwrap().status,MobileGoalStatus::Running);
        assert!(app.execute_mobile_goal_step(&goal.id,runtime.clone()).await.is_err());
        let receipt=app.mobile_workspace_data().await.unwrap().latest_action_receipt.unwrap();
        assert_eq!(receipt.action_id,scroll.action_receipt_id.clone().unwrap());assert!(receipt.command_sent);
        assert_eq!(receipt.status,MobileActionStatus::Executed);
        let proposal=mobile_runtime::planner::MobileCompletionProposal {reason:"One verified public Settings scroll".into(),
            supporting_observation_ids:vec![scroll.observation_before_id.clone().unwrap(),scroll.observation_after_id.clone().unwrap()]};
        assert_eq!(app.verify_mobile_goal_completion(&goal.id,&proposal).await.unwrap().status,MobileGoalStatus::Completed);
        println!("M3_REAL_SCROLL_DOWN=PASS observe_action_observe_verify=PASS completion=PASS duplicate=PASS planner=TEST_ONLY");
        // EXTRACT must change the real persisted feed projection with source/evidence links.
        let before=host.observe_with_sdk(None,&allowed).unwrap();
        let (goal,_)=test_plan(&app,&before,MobileStepType::Extract,ExpectedStepResult::NewDataObject,None,None).await;
        let count=app.mobile_workspace_data().await.unwrap().feed.len();
        let extract=app.execute_mobile_goal_step(&goal.id,runtime.clone()).await.unwrap();assert_trace(&app,&extract,false).await;
        let workspace=app.mobile_workspace_data().await.unwrap();let object=workspace.feed.iter().find(|o|Some(&o.id)==extract.data_object_id.as_ref()).unwrap();
        assert!(workspace.feed.len()>count);assert_eq!(object.data_type,"mobile_observation_object");
        assert_eq!(object.source_method,"MOBILE");assert_eq!(object.mobile_observation_id,extract.observation_before_id);
        assert_eq!(object.evidence_ids,extract.evidence_ids);assert!(!object.evidence_ids.is_empty());
        println!("M3_REAL_EXTRACT=PASS evidence=PASS projection=PASS");
        // INPUT is optional and must use an actual safe, focused editable surface.
        let before=host.observe_with_sdk(None,&allowed).unwrap();
        let search=before.snapshot.elements.iter().find(|e|e.enabled&&e.clickable&&e.content_description.as_deref()==Some("Search"));
        if let Some(search)=search {
            let (goal,_)=test_plan(&app,&before,MobileStepType::TapElement,ExpectedStepResult::UiChanged,Some(search.element_ref.clone()),None).await;
            let tap=app.execute_mobile_goal_step(&goal.id,runtime.clone()).await.unwrap();
            if tap.verified {
                let capture=host.observe_with_sdk(None,&allowed).unwrap();
                let field=capture.snapshot.elements.iter().find(|e|e.enabled&&e.focused&&e.class_name.ends_with("EditText"));
                if mobile_runtime::executor::valid_capture(&capture) && let Some(field)=field {
                    let value="OrdinConn M3 Test".to_owned();
                    let (goal,_)=test_plan(&app,&capture,MobileStepType::InputText,ExpectedStepResult::TextEquals{element_ref:field.element_ref.clone(),value:value.clone()},Some(field.element_ref.clone()),Some(value)).await;
                    let input=app.execute_mobile_goal_step(&goal.id,runtime.clone()).await.unwrap();assert_trace(&app,&input,true).await;
                    assert_eq!(app.mobile_workspace_data().await.unwrap().latest_action_receipt.unwrap().input_value_verified,Some(true));
                    println!("M3_REAL_INPUT_TEXT=PASS exact_value=true");
                } else {println!("M3_REAL_INPUT_TEXT=NOT_AVAILABLE_TEST_SURFACE");}
            } else {println!("M3_REAL_INPUT_TEXT=NOT_AVAILABLE_TEST_SURFACE");}
        } else {println!("M3_REAL_INPUT_TEXT=NOT_AVAILABLE_TEST_SURFACE");}
        assert!(app.snapshot().await.unwrap().providers.is_empty());
        println!("M3_REAL_DEVICE_EXECUTOR_GATE=PASS provider=0 live_planner=NOT_RUN_MODEL_NOT_CONFIGURED");
        app.shutdown().await.unwrap();
    });
    host.stop_session();
}

#[test]
fn real_phase5_input_text_gate_is_explicitly_gated() {
    if std::env::var("ORDINCONN_MOBILE_M3_INPUT_SMOKE").as_deref() != Ok("1") {
        return;
    }
    let _lock = real_avd_gate_lock().expect("exclusive dedicated AVD");
    let host = Arc::new(MobileHost::discover());
    let diagnostics = host.environment_diagnostics(None);
    assert_eq!(diagnostics.online_devices.len(), 1);
    let device = &diagnostics.online_devices[0].id;
    let adb = diagnostics.adb_path.as_deref().unwrap();
    let name = Command::new(adb)
        .args(["-s", device, "emu", "avd", "name"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&name.stdout)
            .lines()
            .any(|s| s.trim() == "OrdinConn_M1_5")
    );
    // Test preparation only: prevent the preinstalled handwriting tutorial intercepting input.
    // Preserve and restore the exact prior setting, including an unset default, on every exit.
    struct KeyboardSurface {
        adb: String,
        device: String,
        prior: String,
        armed: bool,
    }
    impl KeyboardSurface {
        fn restore(&mut self) -> bool {
            if !self.armed {
                return true;
            }
            let mut command = Command::new(&self.adb);
            command.args(["-s", &self.device, "shell", "settings"]);
            if self.prior == "null" {
                command.args(["delete", "secure", "stylus_handwriting_enabled"]);
            } else {
                command.args(["put", "secure", "stylus_handwriting_enabled", &self.prior]);
            }
            let ok = command.output().is_ok_and(|r| r.status.success());
            if ok {
                self.armed = false;
            }
            ok
        }
    }
    impl Drop for KeyboardSurface {
        fn drop(&mut self) {
            if !self.restore() {
                eprintln!("TEST_KEYBOARD_SETTING_RESTORE_FAILED");
            }
        }
    }
    let prior = Command::new(adb)
        .args([
            "-s",
            device,
            "shell",
            "settings",
            "get",
            "secure",
            "stylus_handwriting_enabled",
        ])
        .output()
        .unwrap();
    assert!(prior.status.success());
    let prior = String::from_utf8(prior.stdout).unwrap().trim().to_owned();
    assert!(matches!(prior.as_str(), "null" | "0" | "1"));
    let mut keyboard = KeyboardSurface {
        adb: adb.into(),
        device: device.into(),
        prior,
        armed: true,
    };
    assert!(
        Command::new(adb)
            .args([
                "-s",
                device,
                "shell",
                "settings",
                "put",
                "secure",
                "stylus_handwriting_enabled",
                "0"
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        Command::new(adb)
            .args(["-s", device, "shell", "input", "keyevent", "4"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        Command::new(adb)
            .args([
                "-s",
                device,
                "shell",
                "am",
                "start",
                "-W",
                "--activity-clear-top",
                "-n",
                "com.google.android.settings.intelligence/com.google.android.settings.intelligence.modules.search.SearchActivity"
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    let allowed = vec![
        "com.android.settings".into(),
        "com.google.android.settings.intelligence".into(),
    ];
    // Preparation may wait for focus/tree stabilization, never retry a device action.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let initial = loop {
        if let Ok(capture) = host.observe_with_sdk(None, &allowed)
            && mobile_runtime::executor::valid_capture(&capture)
            && capture.snapshot.elements.iter().any(|e| e.enabled && e.focused && e.class_name.ends_with("EditText") && e.resource_id.as_deref()==Some("com.google.android.settings.intelligence:id/open_search_view_edit_text"))
        {
            break capture;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Settings search input surface unavailable"
        );
        std::thread::sleep(Duration::from_millis(200));
    };
    let dir = tempfile::tempdir().unwrap();
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let app = AppRuntime::initialize(&dir.path().join("phase5-input.sqlite3"))
            .await
            .unwrap();
        app.save_mobile_settings(&MobileRuntimeSettings {
            allowed_apps: allowed.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
        let runtime = Arc::new(DesktopMobileExecutorRuntime::new(
            host.clone(),
            None,
            allowed.clone(),
        ));
        // Surface preparation is separate; INPUT_TEXT uses the real canonical Executor.
        let capture = initial;
        let field = capture
            .snapshot
            .elements
            .iter()
            .find(|e| e.enabled && e.focused && e.class_name.ends_with("EditText"))
            .expect("readable focused non-password system search EditText");
        assert!(mobile_runtime::executor::valid_capture(&capture));
        assert_eq!(
            capture.snapshot.package_name,
            "com.google.android.settings.intelligence"
        );
        assert_eq!(
            field.resource_id.as_deref(),
            Some("com.google.android.settings.intelligence:id/open_search_view_edit_text")
        );
        // Keep random test text inside the existing safe-input policy on every run.
        let nonce =
            format!("n{}", &uuid::Uuid::now_v7().simple().to_string()[27..]).replace('a', "g");
        let value = format!("M3 {nonce}");
        let (g, _) = test_plan(
            &app,
            &capture,
            MobileStepType::InputText,
            ExpectedStepResult::TextEquals {
                element_ref: field.element_ref.clone(),
                value: value.clone(),
            },
            Some(field.element_ref.clone()),
            Some(value.clone()),
        )
        .await;
        let input = app
            .execute_mobile_goal_step(&g.id, runtime.clone())
            .await
            .unwrap();
        assert_trace(&app, &input, true).await;
        let receipt = app
            .mobile_workspace_data()
            .await
            .unwrap()
            .latest_action_receipt
            .unwrap();
        assert_eq!(receipt.input_value_verified, Some(true));
        // Independent tree read proves actual value, beyond the action receipt.
        // Retry only independent read failures; never resend INPUT_TEXT or retry a value mismatch.
        let mut read = None;
        for attempt in 0..3 {
            match runtime.observe().await {
                Ok(capture) => {
                    read = Some(capture);
                    break;
                }
                Err(error) if attempt == 2 => panic!("independent input Observe failed: {error:?}"),
                Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
        let actual = read.expect("independent input observation");
        let actual = actual
            .snapshot
            .elements
            .iter()
            .find(|e| e.resource_id == field.resource_id)
            .unwrap();
        assert_eq!(actual.text.as_deref(), Some(value.as_str()));
        assert!(
            Command::new(adb)
                .args(["-s", device, "shell", "input", "keyevent", "4"])
                .output()
                .unwrap()
                .status
                .success()
        );
        app.shutdown().await.unwrap();
    });
    host.stop_session();
    assert!(keyboard.restore());
    let restored = Command::new(adb)
        .args([
            "-s",
            device,
            "shell",
            "settings",
            "get",
            "secure",
            "stylus_handwriting_enabled",
        ])
        .output()
        .unwrap();
    assert!(restored.status.success());
    assert_eq!(
        String::from_utf8(restored.stdout).unwrap().trim(),
        keyboard.prior
    );
    println!(
        "M3_REAL_INPUT_TEXT=PASS exact_expected_observed=true surface=AOSP_Settings_search executor=REAL planner=TEST_ONLY keyboard_setting_restored=true"
    );
}

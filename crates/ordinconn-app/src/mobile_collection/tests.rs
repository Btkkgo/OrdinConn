use super::*;
use chrono::Utc;
fn observation(id: &str) -> MobileObservation {
    MobileObservation {
        id: id.into(),
        device_id: "emulator-fixture".into(),
        package_name: "com.android.settings".into(),
        activity_name: ".Settings".into(),
        captured_at: Utc::now(),
        screen_width: 100,
        screen_height: 200,
        ui_tree_hash: "hash".into(),
        element_count: 1,
        source: "android_ui_tree".into(),
        previous_observation_id: None,
        redactions: vec![],
        elements: vec![MobileUIElement {
            id: "el_title".into(),
            role: "textview".into(),
            class_name: "android.widget.TextView".into(),
            text: Some("Connected".into()),
            content_description: None,
            resource_id: Some("id/title".into()),
            clickable: false,
            scrollable: false,
            editable: false,
            enabled: true,
            selected: false,
            bounds: mobile_runtime::MobileBounds {
                x: 0,
                y: 0,
                width: 100,
                height: 20,
            },
            redacted: false,
        }],
    }
}
#[tokio::test]
async fn insert_retrieve_deduplicate_and_follow_provenance() {
    let dir = tempfile::tempdir().unwrap();
    let app = AppRuntime::initialize(&dir.path().join("test.sqlite"))
        .await
        .unwrap();
    app.save_mobile_settings(&crate::MobileRuntimeSettings {
        allowed_apps: vec!["com.android.settings".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let a = observation("obs_one");
    app.collect_mobile_observation(&a).await.unwrap();
    let first = app.extract_mobile_observation(&a.id).await.unwrap();
    assert_eq!(first.data_objects.len(), 1);
    assert_eq!(first.last_new_object_count, 1);
    let second = app.extract_mobile_observation(&a.id).await.unwrap();
    assert_eq!(second.last_new_object_count, 0);
    let mut b = observation("obs_two");
    b.previous_observation_id = Some(a.id.clone());
    app.collect_mobile_observation(&b).await.unwrap();
    let next = app.extract_mobile_observation(&b.id).await.unwrap();
    assert_eq!(next.data_objects.len(), 1);
    assert_eq!(next.last_new_object_count, 0);
    assert_eq!(next.observations.len(), 2);
    assert_eq!(next.diffs.len(), 1);
    assert_eq!(next.data_objects[0].provenance.observation_id, "obs_one");
    assert_eq!(
        next.data_objects[0].provenance.element_ids,
        vec!["el_title"]
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM runtime_events WHERE event_type='mobile.data_extracted'",
    )
    .fetch_one(app.pool())
    .await
    .unwrap();
    assert_eq!(count, 3);
}
#[tokio::test]
async fn unrecognized_observation_cannot_extract() {
    let dir = tempfile::tempdir().unwrap();
    let app = AppRuntime::initialize(&dir.path().join("test.sqlite"))
        .await
        .unwrap();
    assert!(app.extract_mobile_observation("missing").await.is_err());
}

#[tokio::test]
async fn repository_filters_and_provenance_keep_original_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let app = AppRuntime::initialize(&dir.path().join("repo.sqlite"))
        .await
        .unwrap();
    app.save_mobile_settings(&crate::MobileRuntimeSettings {
        allowed_apps: vec!["com.android.settings".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let a = observation("original");
    app.collect_mobile_observation(&a).await.unwrap();
    let w = app.extract_mobile_observation(&a.id).await.unwrap();
    let b = observation("repeat");
    app.collect_mobile_observation(&b).await.unwrap();
    app.extract_mobile_observation(&b.id).await.unwrap();
    assert_eq!(
        app.mobile_observation_repository()
            .get_by_device("emulator-fixture")
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        app.mobile_observation_repository()
            .get_by_package("wrong")
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        app.mobile_data_repository()
            .get_by_observation("repeat")
            .await
            .unwrap()
            .len(),
        1
    );
    let source = app
        .mobile_data_provenance(&w.data_objects[0].id)
        .await
        .unwrap();
    assert_eq!(source.observation.id, "original");
    assert_eq!(source.sighting_count, 2);
    assert!(
        source
            .extracted_data
            .iter()
            .all(|d| d.element_id == "el_title")
    );
}
#[tokio::test]
async fn interrupted_manual_actions_fail_closed_without_replay() {
    let dir = tempfile::tempdir().unwrap();
    let app = AppRuntime::initialize(&dir.path().join("restart.sqlite"))
        .await
        .unwrap();
    let r = MobileActionResult {
        action_id: "interrupted".into(),
        action_type: MobileActionType::Tap,
        started_at: Utc::now(),
        completed_at: None,
        status: "running".into(),
        device_id: "emulator-fixture".into(),
        before_observation_id: None,
        after_observation_id: None,
        error_code: None,
        error_message: None,
        triggered_by: "owner".into(),
    };
    app.record_mobile_interaction(&r, "").await.unwrap();
    app.recover_mobile_interactions().await.unwrap();
    let saved = app
        .mobile_action_repository()
        .get_by_id("interrupted")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.status, "failed");
    assert_eq!(saved.error_code.as_deref(), Some("INTERRUPTED_BY_RESTART"));
    assert!(saved.after_observation_id.is_none());
}

#[tokio::test]
async fn action_diff_compares_before_to_stable_after() {
    let dir = tempfile::tempdir().unwrap();
    let app = AppRuntime::initialize(&dir.path().join("diff.sqlite"))
        .await
        .unwrap();
    app.save_mobile_settings(&crate::MobileRuntimeSettings {
        allowed_apps: vec!["com.android.settings".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let a = observation("before");
    let mut b = observation("after");
    b.previous_observation_id = Some(a.id.clone());
    b.elements[0].text = Some("Offline".into());
    let mut c = b.clone();
    c.id = "stable".into();
    c.previous_observation_id = Some(b.id.clone());
    for o in [&a, &b, &c] {
        app.collect_mobile_observation(o).await.unwrap();
    }
    let r = MobileActionResult {
        action_id: "diff-action".into(),
        action_type: MobileActionType::Tap,
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        status: "completed".into(),
        device_id: "emulator-fixture".into(),
        before_observation_id: Some(a.id),
        after_observation_id: Some(c.id.clone()),
        error_code: None,
        error_message: None,
        triggered_by: "owner".into(),
    };
    app.record_mobile_interaction(&r, "com.android.settings")
        .await
        .unwrap();
    let diff = app
        .observation_diff_repository()
        .get_by_id(&c.id)
        .await
        .unwrap()
        .unwrap();
    assert!(diff.changed);
    assert_eq!(diff.before_observation_id, "before");
    assert!(diff.change_types.contains(&"TEXT_CHANGED".into()));
}

#[tokio::test]
async fn repository_rejects_unsanitized_sensitive_observation() {
    let dir = tempfile::tempdir().unwrap();
    let app = AppRuntime::initialize(&dir.path().join("sensitive.sqlite"))
        .await
        .unwrap();
    app.save_mobile_settings(&crate::MobileRuntimeSettings {
        allowed_apps: vec!["com.android.settings".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let mut o = observation("unsafe");
    o.elements[0].text = Some("123456".into());
    o.elements[0].resource_id = Some("id/pin".into());
    assert!(app.collect_mobile_observation(&o).await.is_err());
    assert!(
        app.mobile_observation_repository()
            .get_by_id("unsafe")
            .await
            .unwrap()
            .is_none()
    );
}

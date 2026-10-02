use collector_runtime::*;
#[test]
fn local_settings_observation_can_register_only_as_a_safe_computer_source() {
    let mut source = builtin_sources().remove(0);
    source.id = "android-settings-ui".into();
    source.endpoint = "android://com.android.settings".into();
    source.collector_kind = CollectorKind::Computer;
    source.auth = AuthRequirement::None;
    let mut registry = SourceRegistry::default();
    registry.register(source.clone()).unwrap();
    assert!(registry.get("android-settings-ui").is_some());
    for endpoint in [
        "android://com.private.bank",
        "android://com.android.settings/private",
        "android://user:password@com.android.settings",
    ] {
        source.endpoint = endpoint.into();
        assert!(source.validate_public().is_err());
    }
    source.endpoint = "android://com.android.settings".into();
    source.auth = AuthRequirement::PrivateAccess;
    assert!(source.validate_public().is_err());
    source.auth = AuthRequirement::None;
    source.collector_kind = CollectorKind::Rest;
    assert!(source.validate_public().is_err());
}

#[test]
fn manual_mobile_source_requires_exact_owner_allowlist_and_stays_disabled() {
    let mut source = builtin_sources().remove(0);
    source.id = "manual-ui".into();
    source.endpoint = "android://com.example.public".into();
    source.collector_kind = CollectorKind::Computer;
    source.auth = AuthRequirement::None;
    source.enabled = false;
    let mut registry = SourceRegistry::default();
    assert!(
        registry
            .register_manual_mobile(source.clone(), &["com.example.public".into()])
            .is_ok()
    );
    assert!(registry.get("manual-ui").is_some());
    let mut registry = SourceRegistry::default();
    assert!(
        registry
            .register_manual_mobile(source.clone(), &[])
            .is_err()
    );
    source.enabled = true;
    assert!(
        registry
            .register_manual_mobile(source.clone(), &["com.example.public".into()])
            .is_err()
    );
    source.enabled = false;
    source.endpoint = "android://com.example.public/private".into();
    assert!(
        registry
            .register_manual_mobile(source, &["com.example.public".into()])
            .is_err()
    );
}

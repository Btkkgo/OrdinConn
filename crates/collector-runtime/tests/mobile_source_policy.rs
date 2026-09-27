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

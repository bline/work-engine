// Integration tests compile lifecycle-service without cfg(test), even when the
// native-sim-proof feature is enabled for the executable/library.
#[cfg(feature = "native-sim-proof")]
#[test]
fn production_access_lease_identity_rejects_synthetic_endpoint() {
    use lifecycle_service::NativeProviderRoute;

    let route = NativeProviderRoute::OpenaiChatgptAccessLeaseV1;
    assert_eq!(route.identity(), "openai-chatgpt-access-lease-v1");
    assert_eq!(route.provider_name(), "openai");
    assert_eq!(
        route.configured_base_url(""),
        Some("https://chatgpt.com/backend-api/codex".into())
    );
    assert_eq!(route.configured_base_url("http://127.0.0.1:1234/v1"), None);
}

//! External native-host capability and credential contract.
#![cfg(feature = "external-host")]
use om_kernel::{KernelConfig, Session, protocol::*};
use om_llm::{ProviderKind, Target};
#[test]
fn external_native_transport_uses_ephemeral_keys_without_desktop_store() {
    let mut config = KernelConfig::default();
    config.llm.profiles[0].kind = ProviderKind::Anthropic;
    config.llm.profiles[0].model = "fixture".into();
    config.llm.profiles[0].base_url = "https://fixture.invalid".into();
    config.llm.profiles[0].api_key = Some("synthetic-ios-key".into());
    let mut session = Session::new(config, None);
    session.set_llm_target(Target::Native).unwrap();
    let (reply, _) = session.handle(Request::LlmTestProfile {
        request_id: "probe".into(),
        profile: "deepseek".into(),
        config: None,
    });
    let Response::LlmStarted {
        http: Some(http), ..
    } = reply
    else {
        panic!("expected real native request")
    };
    assert!(
        http.headers
            .iter()
            .any(|(k, v)| k == "x-api-key" && v == "synthetic-ios-key")
    );
    assert!(
        !http
            .headers
            .iter()
            .any(|(k, _)| k == "anthropic-dangerous-direct-browser-access")
    );
}

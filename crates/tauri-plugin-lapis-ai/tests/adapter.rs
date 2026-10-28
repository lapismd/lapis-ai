use std::sync::Arc;
use tauri_plugin_lapis_ai::{
    AcpHost, Config, Error, HostContext, HostPolicy, Launch, Result, Target,
};

struct Policy;
impl HostPolicy for Policy {
    fn authorize(&self, context: &HostContext) -> Result<String> {
        if context.window != "one" || context.url != "tauri://localhost/" {
            return Err(Error("denied"));
        }
        Ok("vault-1".into())
    }
    fn targets(&self, _: &HostContext) -> Result<Vec<Target>> {
        Ok(vec![Target {
            id: "fixture".into(),
            label: "Fixture".into(),
        }])
    }
    fn resolve(&self, _: &HostContext, _: &str, _: Option<&str>) -> Result<Launch> {
        Err(Error("not_prepared"))
    }
}
fn context() -> HostContext {
    HostContext {
        window: "one".into(),
        url: "tauri://localhost/".into(),
    }
}

#[test]
fn synchronous_tauri_construction_keeps_the_existing_admitted_api() {
    let host = Arc::new(AcpHost::new(Arc::new(Policy), Config::default()).unwrap());
    assert_eq!(host.targets(&context()).unwrap()[0].id, "fixture");
    assert!(host.list(&context()).unwrap().is_empty());
    assert_eq!(
        tauri::async_runtime::block_on(host.open(&context(), "open", "fixture", None)).err(),
        Some(Error("not_prepared"))
    );
    tauri::async_runtime::block_on(host.shutdown()).unwrap();
}

#[test]
fn native_caller_and_configuration_failures_remain_closed() {
    let host = AcpHost::new(Arc::new(Policy), Config::default()).unwrap();
    let mut wrong = context();
    wrong.window = "other".into();
    assert_eq!(host.targets(&wrong).err(), Some(Error("denied")));
    let invalid = Config {
        maximum_sessions: 0,
        ..Config::default()
    };
    assert_eq!(
        AcpHost::new(Arc::new(Policy), invalid).err(),
        Some(Error("invalid_host_configuration"))
    );
    tauri::async_runtime::block_on(host.shutdown()).unwrap();
}

#[test]
fn lifecycle_delegation_never_resurrects_a_shutdown_service() {
    let host = AcpHost::new(Arc::new(Policy), Config::default()).unwrap();
    tauri::async_runtime::block_on(host.disable()).unwrap();
    assert_eq!(host.targets(&context()).err(), Some(Error("disabled")));
    host.enable().unwrap();
    assert_eq!(host.targets(&context()).unwrap().len(), 1);
    tauri::async_runtime::block_on(host.revoke("one", None)).unwrap();
    tauri::async_runtime::block_on(host.shutdown()).unwrap();
    assert_eq!(host.enable(), Err(Error("disabled")));
}

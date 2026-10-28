use lapis_ai_service::{Config, HostContext, HostPolicy, Result, SessionInfo};
use std::{ops::Deref, sync::Arc};

/// Tauri supplies its live runtime, including construction from synchronous setup.
pub struct AcpHost(Arc<lapis_ai_service::AcpHost>);
impl AcpHost {
    pub fn new(policy: Arc<dyn HostPolicy>, config: Config) -> Result<Self> {
        Ok(Self(Arc::new(lapis_ai_service::AcpHost::with_runtime(
            policy,
            config,
            tauri::async_runtime::handle().inner().clone(),
        )?)))
    }
    pub async fn open(
        &self,
        context: &HostContext,
        request: &str,
        target: &str,
        resume_binding: Option<String>,
    ) -> Result<SessionInfo> {
        self.0.open(context, request, target, resume_binding).await
    }
}
impl Deref for AcpHost {
    type Target = lapis_ai_service::AcpHost;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

use crate::{AcpHost, Error, HostContext, Result, SessionInfo, StreamMessage, Target};
use lapis_ai_runtime::{SessionRequest, SessionSnapshot};
use std::sync::Arc;
use tauri::{State, WebviewWindow, ipc::Channel};
fn context<R: tauri::Runtime>(window: &WebviewWindow<R>) -> Result<HostContext> {
    Ok(HostContext {
        window: window.label().into(),
        url: window.url().map_err(|_| Error("denied"))?.to_string(),
    })
}
#[tauri::command]
pub fn targets<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
) -> Result<Vec<Target>> {
    state.targets(&context(&window)?)
}
#[tauri::command]
pub fn list<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
) -> Result<Vec<SessionInfo>> {
    state.list(&context(&window)?)
}
#[tauri::command]
pub async fn open<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
    request_id: String,
    target_id: String,
    resume_binding: Option<String>,
) -> Result<SessionInfo> {
    state
        .inner()
        .clone()
        .open(&context(&window)?, &request_id, &target_id, resume_binding)
        .await
}
#[tauri::command]
pub async fn submit<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
    session_id: String,
    request_id: String,
    request: SessionRequest,
) -> Result<()> {
    state
        .submit(&context(&window)?, &session_id, request_id, request)
        .await
}
#[tauri::command]
pub fn snapshot<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
    session_id: String,
    after: u64,
) -> Result<SessionSnapshot> {
    state.snapshot(&context(&window)?, &session_id, after)
}
#[tauri::command]
pub fn attach<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
    session_id: String,
    after: u64,
    on_frame: Channel<StreamMessage>,
) -> Result<String> {
    state.attach(
        &context(&window)?,
        &session_id,
        after,
        Arc::new(move |frame| on_frame.send(frame).map_err(|_| ())),
    )
}
#[tauri::command]
pub fn detach<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
    session_id: String,
    subscription_id: String,
) -> Result<()> {
    state.detach(&context(&window)?, &session_id, &subscription_id)
}
#[tauri::command]
pub fn ack<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
    session_id: String,
    subscription_id: String,
    sequence: u64,
) -> Result<()> {
    state.acknowledge(&context(&window)?, &session_id, &subscription_id, sequence)
}
#[tauri::command]
pub async fn close<R: tauri::Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<AcpHost>>,
    session_id: String,
) -> Result<()> {
    state.close(&context(&window)?, &session_id).await
}

//! Real encrypted peer transport against a fixture inference source.
use super::*;
use iroh::{EndpointAddr, TransportAddr};
async fn shared(seed: u8) -> Arc<Shared> {
    let endpoint = peers::bind(&hex::encode([seed; 32])).await.unwrap();
    Arc::new(Shared {
        config: RwLock::new(Config {
            model: Some("fixture".into()),
            ..Default::default()
        }),
        engine: Mutex::new(None),
        source: RwLock::new(None),
        source_model: RwLock::new(Some("fixture".into())),
        endpoint,
        connections: Mutex::new(BTreeMap::new()),
        slots: Semaphore::new(2),
        peer_slots: Arc::new(Semaphore::new(8)),
        token: "fixture-token-value".into(),
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap(),
    })
}
fn invitation(shared: &Shared) -> Value {
    let socket = shared
        .endpoint
        .bound_sockets()
        .into_iter()
        .find(|a| a.is_ipv4())
        .unwrap();
    serde_json::to_value(EndpointAddr::from_parts(
        shared.endpoint.id(),
        [TransportAddr::Ip(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            socket.port(),
        )))],
    ))
    .unwrap()
}
#[tokio::test]
async fn paired_inference_requires_mutual_admission_and_sharing_off_preserves_local_inference() {
    let host = shared(41).await;
    let client = shared(42).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    *host.source.write().await = Some(format!("http://{}", listener.local_addr().unwrap()));
    let source = tokio::spawn(
        axum::serve(
            listener,
            axum::Router::new().route(
                "/chat/completions",
                axum::routing::post(|| async {
                    axum::Json(json!({"choices":[{"message":{"content":"fixture inference"}}]}))
                }),
            ),
        )
        .into_future(),
    );
    let peer_task = tokio::spawn(peers::listen(host.clone()));
    let host_id = host.endpoint.id().to_string();
    let client_id = client.endpoint.id().to_string();
    client
        .config
        .write()
        .await
        .peers
        .insert(host_id.clone(), invitation(&host));
    let body = br#"{"model":"fixture","messages":[],"stream":false}"#;
    assert!(
        peers::infer(&client, &host_id, "chat/completions", body)
            .await
            .is_err()
    );
    {
        let mut config = host.config.write().await;
        config.peers.insert(client_id.clone(), invitation(&client));
        config.sharing = true;
    }
    assert!(
        peers::infer(&client, &host_id, "chat/completions", body)
            .await
            .unwrap()
            .1
            .contains("fixture inference")
    );
    host.config.write().await.sharing = false;
    peers::revoke_connections(&host).await;
    assert!(
        peers::infer(&client, &host_id, "chat/completions", body)
            .await
            .is_err()
    );
    assert!(
        gateway::infer(&host, "chat/completions", body)
            .await
            .unwrap()
            .1
            .contains("fixture inference")
    );
    host.endpoint.close().await;
    client.endpoint.close().await;
    peer_task.abort();
    source.abort();
}

#[tokio::test]
#[ignore = "requires the explicitly prepared isolated native runtime and GGUF weights"]
async fn native_installed_model_starts_and_serves_without_public_discovery() {
    let root = std::env::var_os("JJ_STORE_COMPUTE_TEST_ROOT").expect("explicit isolated test root");
    std::env::set_current_dir(root).unwrap();
    struct Diagnostics;
    impl mesh_llm_events::OutputSink for Diagnostics {
        fn emit_event(&self, event: mesh_llm_events::OutputEvent) -> std::io::Result<()> {
            match event {
                mesh_llm_events::OutputEvent::Warning { message, context }
                | mesh_llm_events::OutputEvent::Error { message, context }
                | mesh_llm_events::OutputEvent::Fatal { message, context } => {
                    eprintln!("native acceptance: {message}; {context:?}");
                }
                _ => {}
            }
            Ok(())
        }
        fn console_session_mode(&self) -> Option<mesh_llm_events::ConsoleSessionMode> {
            Some(mesh_llm_events::ConsoleSessionMode::InteractiveDashboard)
        }
    }
    mesh_llm_events::set_output_sink(Arc::new(Diagnostics));
    let mut host = shared(88).await;
    Arc::get_mut(&mut host).unwrap().client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap();
    host.config.write().await.model = Some(
        std::env::var("LAPIS_AI_NATIVE_TEST_MODEL")
            .unwrap_or_else(|_| "unsloth/Qwen3-0.6B-GGUF:Q4_K_M".into()),
    );
    host.config.write().await.memory_gb = Some(4);
    let started = engine::start(&host).await;
    if let Err(error) = started {
        host.endpoint.close().await;
        panic!("native installed startup: {error:#}");
    }
    let response = gateway::infer(&host, "chat/completions", br#"{"model":"auto","messages":[{"role":"user","content":"Reply with LOCAL_READY."}],"max_tokens":64,"stream":false,"chat_template_kwargs":{"enable_thinking":false}}"#).await;
    // Release native resources even when inference fails, before asserting.
    host.engine
        .lock()
        .await
        .take()
        .unwrap()
        .stop()
        .await
        .unwrap();
    host.endpoint.close().await;
    let response = response.expect("native inference");
    assert_eq!(response.0, 200);
    let value: Value = serde_json::from_str(&response.1).unwrap();
    assert!(
        !value["choices"][0]["message"]["content"]
            .as_str()
            .unwrap()
            .is_empty()
    );
}

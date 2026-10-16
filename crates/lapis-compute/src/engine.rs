//! Exact native runtime selection and explicit model lifecycle.
use crate::{Shared, emit};
use mesh_llm_sdk::{MeshDiscoveryMode, serve};
use serde_json::json;
use std::time::Duration;
fn prepared_runtime() -> anyhow::Result<bool> {
    use mesh_llm_sdk::native_runtime::*;
    let cache = default_native_runtime_cache()?;
    let bundled = std::env::var_os("MESH_LLM_NATIVE_RUNTIME_BUNDLE_DIR")
        .map(std::path::PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();
    Ok(discover_local_native_runtimes(&bundled, &cache)?
        .iter()
        .any(|runtime| {
            native_runtime_versions_match_current_sdk(
                &runtime.mesh_version,
                &runtime.manifest.runtime.skippy_abi,
            ) && runtime.load_plan().is_ok()
        }))
}
pub async fn prepare() -> anyhow::Result<()> {
    if prepared_runtime()? {
        return Ok(());
    }
    use mesh_llm_sdk::native_runtime::*;
    let runtime = install_native_runtime(NativeRuntimeInstallOptions {
        skippy_abi_version: Some(current_skippy_abi_version()),
        ..Default::default()
    })
    .await?
    .runtime;
    anyhow::ensure!(
        native_runtime_versions_match_current_sdk(
            &runtime.mesh_version,
            &runtime.manifest.runtime.skippy_abi
        ),
        "runtime mismatch"
    );
    runtime.load_plan()?;
    Ok(())
}
pub async fn download(shared: &Shared) -> anyhow::Result<()> {
    anyhow::ensure!(shared.engine.lock().await.is_none(), "model is running");
    let model = shared
        .config
        .read()
        .await
        .model
        .clone()
        .ok_or_else(|| anyhow::anyhow!("model required"))?;
    let download =
        mesh_llm_host_runtime::models::download_model_ref_with_progress_details(&model, true)
            .await?;
    crate::models::record(&model, download.path, download.paths)?;
    Ok(())
}
fn port() -> anyhow::Result<u16> {
    Ok(std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port())
}
pub async fn start(shared: &Shared) -> anyhow::Result<()> {
    let mut slot = shared.engine.lock().await;
    anyhow::ensure!(slot.is_none(), "already running");
    let config = shared.config.read().await.clone();
    let model = config
        .model
        .ok_or_else(|| anyhow::anyhow!("model required"))?;
    let installed = crate::models::installed(&model)?;
    // Startup verifies a previously prepared/bundled artifact; it never installs one.
    anyhow::ensure!(prepared_runtime()?, "native runtime preparation required");
    // Embedded serving loads only local artifacts. The general host initializer
    // can install a runtime, so it must not run during explicit model startup.
    // The gateway preserves logical identity; native startup uses installed weights.
    // Bound native context/slots and prohibit implicit source resolution.
    let installed_path = installed.to_string_lossy().into_owned();
    let native_config = toml::toml! {
        [[plugin]]
        name = "telemetry"
        enabled = false
        [[plugin]]
        name = "blobstore"
        enabled = false
        [defaults]
        ctx_size = 8192
        parallel = 2
        [defaults.skippy]
        source_policy = "local-required"
    };
    let mut config_file = tempfile::NamedTempFile::new()?;
    use std::io::Write;
    config_file.write_all(native_config.to_string().as_bytes())?;
    let mut builder = serve::EmbeddedServeConfig::builder()
        .model(installed_path)
        .config_path(config_file.path())
        .api_port(port()?)
        .console_port(port()?)
        .console_ui(false)
        .publish(false)
        .auto_join(false)
        .discovery_mode(MeshDiscoveryMode::Nostr)
        .disable_iroh_relays(true)
        .enumerate_host(false)
        .listen_all(false)
        .bind_ip("127.0.0.1".parse()?)
        .isolated_config(true)
        .startup_timeout(Duration::from_secs(180));
    if let Some(memory) = config.memory_gb {
        builder = builder.max_vram_gb(memory as f64);
    }
    let handle = serve::start(builder.build()).await?;
    let origin = handle.api_base_url().to_owned();
    let advertised = async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
        loop {
            if let Ok(response) = shared
                .client
                .get(format!("{origin}/models"))
                .timeout(Duration::from_secs(5))
                .send()
                .await
                && let Ok(response) = response.error_for_status()
                && let Ok(value) = response.json::<serde_json::Value>().await
                && let Some(id) = value["data"][0]["id"].as_str()
            {
                break Ok::<_, anyhow::Error>(id.to_owned());
            }
            anyhow::ensure!(
                tokio::time::Instant::now() < deadline,
                "inference model unavailable"
            );
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    .await;
    let advertised = match advertised {
        Ok(id) => id,
        Err(error) => {
            let _ = tokio::time::timeout(Duration::from_secs(12), handle.stop()).await;
            return Err(error);
        }
    };
    *shared.source_model.write().await = Some(advertised);
    *shared.source.write().await = Some(origin);
    *slot = Some(handle);
    emit(json!({"type":"serving"}));
    Ok(())
}

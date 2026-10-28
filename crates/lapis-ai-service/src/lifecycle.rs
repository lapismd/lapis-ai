use crate::{AcpHost, Error, Result, cleanup, registry::Slot};
impl AcpHost {
    pub fn enable(&self) -> Result<()> {
        let mut state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
        if state.stopped {
            return Err(Error("disabled"));
        }
        state.enabled = true;
        Ok(())
    }
    pub async fn disable(&self) -> Result<()> {
        self.stop(false).await
    }
    pub async fn shutdown(&self) -> Result<()> {
        self.stop(true).await
    }
    async fn stop(&self, permanent: bool) -> Result<()> {
        let completions = {
            let mut state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
            state.enabled = false;
            state.stopped |= permanent;
            let slots = state.slots.drain().map(|(_, slot)| slot).collect();
            cleanup::retire(&self.runtime, &mut state, &self.state, slots);
            cleanup::pending(&state, |_| true)
        };
        cleanup::wait(completions).await
    }
    /// Rust-only owner teardown; unrelated windows and vaults remain alive.
    pub async fn revoke(&self, window: &str, admission: Option<&str>) -> Result<()> {
        let completions = {
            let mut state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
            let ids: Vec<_> = state
                .slots
                .iter()
                .filter(|(_, slot)| {
                    slot.scope.window == window
                        && admission.is_none_or(|id| slot.scope.admission == id)
                })
                .map(|(id, _)| id.clone())
                .collect();
            let slots = ids
                .into_iter()
                .filter_map(|id| state.slots.remove(&id))
                .collect();
            cleanup::retire(&self.runtime, &mut state, &self.state, slots);
            cleanup::pending(&state, |scope| {
                scope.window == window && admission.is_none_or(|id| scope.admission == id)
            })
        };
        cleanup::wait(completions).await
    }
    pub(crate) async fn stop_slots(slots: Vec<Slot>) -> Result<()> {
        let mut tasks = tokio::task::JoinSet::new();
        for mut slot in slots {
            slot.cancel.send_replace(true);
            slot.subscription.take();
            if let Some(session) = slot.session.take() {
                tasks.spawn(async move { session.close().await });
            }
            if let Some(mut done) = slot.startup_done.take() {
                tasks.spawn(async move {
                    tokio::time::timeout(std::time::Duration::from_secs(30), async {
                        loop {
                            if let Some(result) = *done.borrow_and_update() {
                                return result
                                    .map_err(|_| lapis_ai_runtime::Error("cleanup_unknown"));
                            }
                            done.changed()
                                .await
                                .map_err(|_| lapis_ai_runtime::Error("cleanup_unknown"))?;
                        }
                    })
                    .await
                    .map_err(|_| lapis_ai_runtime::Error("cleanup_unknown"))?
                });
            }
        }
        let mut failed = false;
        while let Some(result) = tasks.join_next().await {
            failed |= !matches!(result, Ok(Ok(())));
        }
        if failed {
            Err(Error("cleanup_unknown"))
        } else {
            Ok(())
        }
    }
}
impl Drop for AcpHost {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.enabled = false;
            state.stopped = true;
            let slots = state.slots.drain().map(|(_, slot)| slot).collect();
            cleanup::retire(&self.runtime, &mut state, &self.state, slots);
        }
    }
}

use crate::{
    AcpHost, Error, Result,
    registry::{Registry, Retirement, Scope, Slot},
};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use uuid::Uuid;

pub(crate) type Completion = watch::Receiver<Option<Result<()>>>;

/// Register while holding the same lock used to remove active slots. The job,
/// rather than any calling future, owns the workers until supervised cleanup.
pub(crate) fn retire(
    state: &mut Registry,
    owner: &Arc<Mutex<Registry>>,
    mut slots: Vec<Slot>,
) -> Vec<Completion> {
    if slots.is_empty() {
        return vec![];
    }
    for slot in &mut slots {
        slot.cancel.send_replace(true);
        slot.subscription.take();
    }
    let id = Uuid::new_v4().to_string();
    let (done, completion) = watch::channel(None);
    state.retiring.insert(
        id.clone(),
        Retirement {
            scopes: slots.iter().map(|slot| slot.scope.clone()).collect(),
            done: completion.clone(),
        },
    );
    let owner = owner.clone();
    tauri::async_runtime::spawn(async move {
        let result = AcpHost::stop_slots(slots).await;
        if result.is_ok()
            && let Ok(mut state) = owner.lock()
        {
            state.retiring.remove(&id);
        }
        // Failures remain reserved and observable to later revoke/exit calls.
        done.send_replace(Some(result));
    });
    vec![completion]
}

pub(crate) fn pending(state: &Registry, matches: impl Fn(&Scope) -> bool) -> Vec<Completion> {
    state
        .retiring
        .values()
        .filter(|job| job.scopes.iter().any(&matches))
        .map(|job| job.done.clone())
        .collect()
}

pub(crate) async fn wait(completions: Vec<Completion>) -> Result<()> {
    let mut failed = false;
    for mut done in completions {
        let result = tokio::time::timeout(std::time::Duration::from_secs(35), async {
            loop {
                if let Some(result) = *done.borrow_and_update() {
                    return result;
                }
                done.changed().await.map_err(|_| Error("cleanup_unknown"))?;
            }
        })
        .await;
        failed |= !matches!(result, Ok(Ok(())));
    }
    if failed {
        Err(Error("cleanup_unknown"))
    } else {
        Ok(())
    }
}

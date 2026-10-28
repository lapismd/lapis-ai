use super::{journal::Journal, processed::FrameBatch};
use crate::{Error, Result};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::watch, time::Instant};

/// Wait with one already processed group; never re-run capture or grow the journal.
pub async fn emit(
    journal: &Arc<Mutex<Journal>>,
    frame: FrameBatch,
    stop: &mut watch::Receiver<bool>,
    deadline: Instant,
) -> Result<bool> {
    let size = frame.size()?;
    let mut capacity = {
        let mut journal = journal.lock().map_err(|_| Error("session_unavailable"))?;
        // An admitted callback must finish its whole publication before close.
        // Only a full journal introduces a cancellable handoff wait.
        if journal.can_emit(size)? {
            journal.emit(frame)?;
            return Ok(true);
        }
        journal.capacity_changed()
    };
    let until = (Instant::now() + Duration::from_millis(100)).min(deadline);
    loop {
        if *stop.borrow() {
            return Ok(false);
        }
        if Instant::now() >= deadline {
            return Err(Error("outcome_unknown"));
        }
        if Instant::now() >= until {
            return Err(Error("output_coverage_lost"));
        }
        {
            let mut journal = journal.lock().map_err(|_| Error("session_unavailable"))?;
            if journal.can_emit(size)? {
                journal.emit(frame)?;
                return Ok(true);
            }
        }
        tokio::select! {
            biased;
            _ = stop.changed() => return Ok(false),
            _ = tokio::time::sleep_until(until) => return Err(Error(
                if Instant::now() >= deadline { "outcome_unknown" } else { "output_coverage_lost" }
            )),
            result = capacity.changed() => { result.map_err(|_| Error("session_unavailable"))?; }
        }
    }
}

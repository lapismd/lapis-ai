//! Parallel lifecycle tests must never share disposable state.
mod common;
use common::Scratch;
use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};

#[test]
fn concurrent_scratch_directories_survive_peer_teardown() {
    for _ in 0..16 {
        let barrier = Arc::new(Barrier::new(32));
        let mut directories = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..32)
                .map(|_| {
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        Scratch::new("parallel-regression")
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        let paths: BTreeSet<_> = directories
            .iter()
            .map(|directory| directory.join("."))
            .collect();
        assert_eq!(
            paths.len(),
            directories.len(),
            "concurrent tests shared a directory"
        );
        let survivor = directories.pop().unwrap();
        drop(directories);
        assert!(
            survivor.join(".").is_dir(),
            "peer teardown removed live test state"
        );
    }
}

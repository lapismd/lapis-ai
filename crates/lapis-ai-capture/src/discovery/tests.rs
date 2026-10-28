use super::*;
use std::io::{Error, ErrorKind};
#[test]
fn current_binding_distinguishes_absence_from_unknown_io_outcomes() {
    assert!(
        binding_observation::<()>(Err(ErrorKind::NotFound.into()))
            .unwrap()
            .is_none()
    );
    for kind in [
        ErrorKind::PermissionDenied,
        ErrorKind::TimedOut,
        ErrorKind::Other,
    ] {
        let error = binding_observation::<()>(Err(kind.into())).unwrap_err();
        assert_eq!(error.downcast_ref::<Error>().unwrap().kind(), kind);
    }
    assert_eq!(binding_observation(Ok(1)).unwrap(), Some(1));
}

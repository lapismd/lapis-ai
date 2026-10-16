//! Explicit interactive version selection.
use crate::args::ArgError;
pub(crate) fn parse(value: &str) -> Result<u64, ArgError> {
    match value {
        "1" => Ok(1),
        "2" => Ok(2),
        "3" => Ok(3),
        _ => Err(ArgError::Invalid(
            "control version must be 1, 2 or 3".into(),
        )),
    }
}

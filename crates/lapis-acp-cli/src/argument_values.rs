//! Bounded scalar argument parsing shared by launch options.
use crate::args::{ArgError, Env, MAX_WORKERS, duration};
use std::time::Duration;
/// Refuse arguments that no subcommand would read.
///
/// Silently ignoring them is worse than refusing: `agents --catalog x` that drops
/// the catalog reports a runner set the operator did not ask for, and looks like a
/// correct answer.
pub(crate) fn reject_trailing(items: &mut std::vec::IntoIter<String>) -> Result<(), ArgError> {
    match items.next() {
        Some(argument) => Err(ArgError::Usage(format!("unexpected argument {argument:?}"))),
        None => Ok(()),
    }
}

pub(crate) fn next_value(
    items: &mut std::vec::IntoIter<String>,
    flag: &str,
) -> Result<String, ArgError> {
    items
        .next()
        .ok_or_else(|| ArgError::Usage(format!("{flag} needs a value")))
}

pub(crate) fn count(value: &str, option: &str) -> Result<usize, ArgError> {
    let parsed: usize = value
        .trim()
        .parse()
        .map_err(|_| ArgError::Invalid(format!("{option} must be a whole number")))?;
    if parsed == 0 {
        return Err(ArgError::Invalid(format!("{option} must be at least 1")));
    }
    if parsed > MAX_WORKERS && option == "--workers" {
        return Err(ArgError::Invalid(format!(
            "{option} must be at most {MAX_WORKERS}"
        )));
    }
    Ok(parsed)
}

pub(crate) fn optional_count(env: &Env, name: &str, flag: &str) -> Result<Option<usize>, ArgError> {
    match env(name) {
        Some(value) => Ok(Some(count(&value, flag)?)),
        None => Ok(None),
    }
}

pub(crate) fn optional_duration(
    env: &Env,
    name: &str,
    flag: &str,
) -> Result<Option<Duration>, ArgError> {
    match env(name) {
        Some(value) => Ok(Some(duration(&value, flag)?)),
        None => Ok(None),
    }
}

//! Catalog failures.
//!
//! Every variant names what was refused and why. Resolution never falls back to
//! a default runner, because silently substituting a different agent for the one
//! a controlling application asked for would run untrusted prompt content with
//! the wrong capabilities.

/// Errors produced while declaring or resolving a runner.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The identifier is empty, malformed or too long.
    #[error("invalid runner identifier {0:?}")]
    InvalidId(String),

    /// The label is empty or too long.
    #[error("runner {0} has an invalid label")]
    InvalidLabel(String),

    /// A declaration supplied no executable, or an empty argument.
    #[error("runner {0} declares no usable command")]
    EmptyCommand(String),

    /// The MCP transport is neither stdio nor http.
    #[error("runner {0} has an invalid MCP transport")]
    InvalidTransport(String),

    /// Two declarations in the same tier claim one identifier.
    #[error("duplicate runner {0}")]
    Duplicate(String),

    /// A lower tier tried to claim a compiled-in identifier.
    #[error("runner {0} is reserved and cannot be overridden")]
    Reserved(String),

    /// No declaration carries this identifier.
    #[error("unknown runner {0}")]
    Unknown(String),

    /// The identifier resolved to a runner that is not enabled.
    #[error("runner {0} is not enabled")]
    Disabled(String),

    /// A catalog document was malformed.
    #[error("invalid runner catalog: {0}")]
    Parse(String),
}

/// The result type used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;

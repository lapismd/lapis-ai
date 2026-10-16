//! Launch environment assembly and reserved-key protection.
//!
//! Precedence is inherited environment, then the definition floor, then caller
//! overrides. Reserved keys are stripped from all three, because the harness
//! owns them and a runner definition must not be able to claim harness identity.

use lapis_acp_protocol::EnvVar;

/// Prefix of the namespace the harness injects into a spawned agent.
///
/// This is deliberately narrow. It reserves only the task identity the harness
/// itself sets, not the whole `JJ_ACP_` tree: that wider tree also carries
/// launch options an operator sets on purpose, such as `JJ_ACP_RUNNER` and
/// `JJ_ACP_MODEL`, and stripping those would break configuration rather than
/// protect identity.
pub const RESERVED_PREFIX: &str = "JJ_ACP_TASK_";

/// Report whether a variable name is owned by the harness.
///
/// Matching is case-sensitive and prefix-based, so `JJ_ACP_TASK_ID` and any
/// future task-scoped harness variable are covered without enumerating them.
pub fn is_reserved_env(name: &str) -> bool {
    name.starts_with(RESERVED_PREFIX)
}

/// One assembled launch environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentPlan {
    /// Variables to set on the spawned agent, in a stable order.
    pub variables: Vec<EnvVar>,
    /// Reserved names that were removed, in the order they were seen.
    pub stripped: Vec<String>,
}

impl EnvironmentPlan {
    /// Look up one variable's value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.variables
            .iter()
            .find(|variable| variable.name == name)
            .map(|variable| variable.value.as_str())
    }
}

/// Assemble a launch environment from its three precedence layers.
///
/// A later layer replaces an earlier one for the same name rather than
/// duplicating it, and the first appearance fixes the position so the result is
/// deterministic. Reserved names are removed from every layer and reported, so an
/// operator can see that a definition tried to claim harness identity instead of
/// having the attempt silently ignored.
pub fn plan(
    inherited: &[(String, String)],
    floor: &[EnvVar],
    overrides: &[(String, String)],
) -> EnvironmentPlan {
    let mut order: Vec<String> = Vec::new();
    let mut values: std::collections::BTreeMap<String, String> = Default::default();
    let mut stripped: Vec<String> = Vec::new();

    let mut layer = |pairs: &mut dyn Iterator<Item = (String, String)>| {
        for (name, value) in pairs {
            if is_reserved_env(&name) {
                if !stripped.contains(&name) {
                    stripped.push(name);
                }
                continue;
            }
            if !values.contains_key(&name) {
                order.push(name.clone());
            }
            values.insert(name, value);
        }
    };

    layer(&mut inherited.iter().cloned());
    layer(&mut floor.iter().map(|v| (v.name.clone(), v.value.clone())));
    layer(&mut overrides.iter().cloned());

    EnvironmentPlan {
        variables: order
            .into_iter()
            .map(|name| {
                let value = values.remove(&name).unwrap_or_default();
                EnvVar::new(name, value)
            })
            .collect(),
        stripped,
    }
}

/// The harness-owned variables a spawned agent receives.
///
/// These are produced only here, so no catalog file and no caller-supplied
/// override can set them. A task identifier travels to the agent for correlation;
/// nothing here is a credential and nothing here authorizes anything.
pub fn harness_variables(task_id: Option<&str>, runner_id: &str) -> Vec<EnvVar> {
    let mut variables = vec![EnvVar::new(format!("{RESERVED_PREFIX}RUNNER"), runner_id)];
    if let Some(task_id) = task_id {
        variables.push(EnvVar::new(format!("{RESERVED_PREFIX}ID"), task_id));
    }
    variables
}

//! Normalized reasoning-effort matching and parameterized model identifiers.
//!
//! Adapters disagree about where reasoning effort lives and what it is called.
//! One exposes a `thought_level` configuration option; another has no such option
//! and instead encodes effort inside a parameterized model identifier. The
//! attribute key varies too: `effort`, `reasoning_effort`, `thinking` and
//! `variant` all appear in the wild.
//!
//! Normalization here exists to *match*, never to constrain. A real adapter
//! advertises `max` and `ultra`, which are not in any portable vocabulary, so a
//! closed enum would make them unselectable. Callers therefore match a requested
//! value against what a session actually advertised, and the value sent back is
//! always the adapter's own spelling.

/// A portable reasoning-effort level, used only for matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thinking {
    /// Reasoning disabled. Adapters may spell this `off` or `none`.
    Off,
    /// Low effort.
    Low,
    /// Medium effort.
    Medium,
    /// High effort.
    High,
    /// Extra high effort.
    XHigh,
}

impl Thinking {
    /// The canonical spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
        }
    }

    /// The operator-facing label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::XHigh => "Extra High",
        }
    }

    /// Normalize an adapter's or a caller's spelling.
    ///
    /// Matching is case-insensitive and treats `none` as `off`, because adapters
    /// disagree about which of the two they send. Anything outside the portable
    /// vocabulary returns `None` rather than being coerced, so `max` and `ultra`
    /// remain selectable by exact match.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "none" => Some(Self::Off),
            "low" | "minimal" => Some(Self::Low),
            "medium" | "med" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" | "x-high" | "extra high" => Some(Self::XHigh),
            _ => None,
        }
    }
}

/// Model identifier attribute keys that name a reasoning *level*, in priority
/// order.
///
/// Real adapter output uses more than one spelling for the same idea, so all are
/// consulted. `effort` and `reasoning_effort` both appear across candidates from a
/// single adapter, which is why neither can be assumed.
pub const EFFORT_LEVEL_KEYS: [&str; 3] = ["effort", "reasoning_effort", "variant"];

/// The attribute key that marks reasoning as present without naming a level.
pub const THINKING_KEY: &str = "thinking";

/// Every attribute key that carries reasoning information.
pub const EFFORT_KEYS: [&str; 4] = ["effort", "reasoning_effort", "variant", "thinking"];

/// The attribute key that marks a low-latency variant.
pub const FAST_KEY: &str = "fast";

/// Report whether two effort spellings mean the same level.
///
/// Exact match wins, so an adapter-specific value such as `max` matches itself
/// even though no portable level names it. Normalized match is the fallback, so
/// a caller asking for `off` matches an adapter that advertises `none`.
pub fn effort_matches(advertised: &str, requested: &str) -> bool {
    if advertised == requested {
        return true;
    }
    match (Thinking::parse(advertised), Thinking::parse(requested)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

/// The base model name, without any bracketed parameters.
pub fn model_id_base(raw: &str) -> &str {
    match raw.find('[') {
        Some(index) => raw[..index].trim_end(),
        None => raw,
    }
}

/// Parse the bracketed parameters of a model identifier.
///
/// An identifier without brackets yields an empty list. A malformed segment is
/// skipped rather than rejected, because these strings are adapter-authored and
/// an unparseable attribute must not make the whole model unselectable.
pub fn model_id_attrs(raw: &str) -> Vec<(String, String)> {
    let Some(open) = raw.find('[') else {
        return Vec::new();
    };
    let Some(close) = raw.rfind(']') else {
        return Vec::new();
    };
    if close <= open {
        return Vec::new();
    }
    raw[open + 1..close]
        .split(',')
        .filter_map(|part| {
            let separator = part.find('=')?;
            if separator == 0 {
                return None;
            }
            let key = part[..separator].trim();
            let value = part[separator + 1..].trim();
            if key.is_empty() {
                return None;
            }
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

/// The reasoning effort a parameterized model identifier advertises, verbatim.
///
/// A level-bearing key wins over `thinking`, because `thinking=true` asserts that
/// reasoning happens without saying how much. An identifier carrying both is
/// therefore reported at its level, and one carrying only `thinking=true` is
/// reported as `true` so a caller can still tell it reasons.
///
/// Precedence follows [`EFFORT_LEVEL_KEYS`] rather than the order the attributes
/// happen to appear in, so `model[thinking=true,effort=high]` reports `high`.
pub fn advertised_effort(raw: &str) -> Option<String> {
    let attrs = model_id_attrs(raw);
    for key in EFFORT_LEVEL_KEYS {
        if let Some((_, value)) = attrs.iter().find(|(name, _)| name == key) {
            return Some(value.clone());
        }
    }
    attrs
        .iter()
        .find(|(key, _)| key == THINKING_KEY)
        .map(|(_, value)| value.clone())
}

/// Report whether a model identifier is marked as a low-latency variant.
pub fn advertises_fast(raw: &str) -> bool {
    model_id_attrs(raw)
        .iter()
        .any(|(key, value)| key == FAST_KEY && value == "true")
}

/// Operator-facing badges for a parameterized model identifier.
///
/// Presentation only: a badge never changes what is sent to an adapter.
pub fn badges(raw: &str) -> Vec<&'static str> {
    let mut result = Vec::new();
    if advertises_fast(raw) {
        result.push("Fast");
    }
    if let Some(effort) = advertised_effort(raw) {
        match Thinking::parse(&effort) {
            Some(level) => result.push(level.label()),
            None if effort == "true" => result.push("Thinking"),
            None => {}
        }
    }
    result
}

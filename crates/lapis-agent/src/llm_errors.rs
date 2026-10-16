// Derived from the maintained agent fork; unchanged response classification.
pub(super) fn is_context_length_error(body: &str) -> bool {
    let b = body.to_ascii_lowercase();
    // OpenAI/Databricks machine-readable code; the most reliable marker.
    b.contains("context_length_exceeded")
        // Prose forms: OpenAI's classic phrasing and the Databricks gateway's
        // "context window of this model" variant seen in both bug reports.
        || b.contains("maximum context length")
        || b.contains("context window")
        // Anthropic: "prompt is too long: N tokens > M maximum".
        || b.contains("prompt is too long")
}

/// Narrow matcher for "you should be on the Responses API" provider errors,
/// the signal we use to auto-upgrade. Triggers on the literal path
/// `/v1/responses` (Databricks GPT-5.5 phrasing) or the prose
/// "use the Responses API" / "Responses API instead".
pub(super) fn is_responses_required_error(body: &str) -> bool {
    let b = body.to_ascii_lowercase();
    b.contains("/v1/responses")
        || b.contains("responses api instead")
        || b.contains("use the responses api")
}

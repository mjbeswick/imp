//! The model's context window, and how full the context is against it.

/// Window sizes by model name, each with the source it was read from.
/// Models not listed here take `context_window` from the config, or show no
/// percentage.
const WINDOWS: &[(&str, u64)] = &[
    // DeepSeek Models & Pricing page, "CONTEXT LENGTH 1M", read 23 Nov 2026:
    // https://api-docs.deepseek.com/quick_start/pricing
    ("deepseek-flash", 1_000_000),
    // OpenAI GPT-4.1 mini model page, "1,047,576 context window", checked 25 Nov 2026:
    // https://developers.openai.com/api/docs/models/gpt-4.1-mini
    ("gpt-4.1-mini", 1_047_576),
    // Google Gemini 2.5 Pro, Gemini Enterprise Agent Platform docs, "Context window 1,048,576", checked 25 Nov 2026:
    // https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/gemini/2-5-pro
    ("gemini-2.5-pro", 1_048_576),
];

/// The window for `model`: the configured size when set, else the table, else
/// nothing.
pub fn window(model: &str, configured: Option<u64>) -> Option<u64> {
    configured.or_else(|| {
        WINDOWS
            .iter()
            .find(|(name, _)| *name == model)
            .map(|(_, tokens)| *tokens)
    })
}

/// The share of `window` that `tokens` fills, in whole percent, rounded down
/// and capped at 100. Nothing for an unknown window.
pub fn percent(tokens: u64, window: Option<u64>) -> Option<u64> {
    let window = window.filter(|w| *w > 0)?;
    Some((tokens.saturating_mul(100) / window).min(100))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_configured_window_wins_over_the_table() {
        assert_eq!(window("deepseek-flash", Some(32_000)), Some(32_000));
    }

    #[test]
    fn the_table_answers_when_nothing_is_configured() {
        assert_eq!(window("deepseek-flash", None), Some(1_000_000));
    }

    #[test]
    fn an_unknown_model_has_no_window() {
        assert_eq!(window("some-other-model", None), None);
    }

    #[test]
    fn the_table_has_no_duplicate_names() {
        let mut names: Vec<&str> = WINDOWS.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), WINDOWS.len());
        assert!(!WINDOWS.is_empty());
    }

    #[test]
    fn percent_rounds_down() {
        assert_eq!(percent(12_300, Some(200_000)), Some(6));
        assert_eq!(percent(11_999, Some(200_000)), Some(5));
    }

    #[test]
    fn percent_is_capped_at_one_hundred() {
        assert_eq!(percent(250_000, Some(200_000)), Some(100));
    }

    #[test]
    fn percent_is_nothing_without_a_window() {
        assert_eq!(percent(7_700, None), None);
    }

    #[test]
    fn percent_of_zero_tokens_is_zero() {
        assert_eq!(percent(0, Some(200_000)), Some(0));
    }

    #[test]
    fn the_built_in_gpt_and_gemini_models_have_a_window() {
        assert_eq!(window("gpt-4.1-mini", None), Some(1_047_576));
        assert_eq!(window("gemini-2.5-pro", None), Some(1_048_576));
    }

    #[test]
    fn a_configured_window_overrides_the_table_and_an_unlisted_model_has_none() {
        assert_eq!(window("gpt-4.1-mini", Some(128_000)), Some(128_000));
        assert_eq!(window("some-unlisted-model", None), None);
    }
}

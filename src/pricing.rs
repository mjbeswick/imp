//! DeepSeek prices, per million tokens. Peak doubles, in UTC.

#[derive(Clone, Copy)]
pub struct Prices {
    pub hit: f64,
    pub miss: f64,
    pub output: f64,
}

/// Peak is 01:00–04:00 and 06:00–10:00 UTC, Monday to Friday.
pub fn is_peak(unix_seconds: u64) -> bool {
    let day = unix_seconds / 86_400;
    let hour = (unix_seconds % 86_400) / 3_600;
    // 1970-01-01 was a Thursday; 0 = Sunday.
    let weekday = (day + 4) % 7;
    let weekend = weekday == 0 || weekday == 6;
    !weekend && ((1..4).contains(&hour) || (6..10).contains(&hour))
}

pub fn prices(model: &str, unix_seconds: u64) -> Option<Prices> {
    let peak = is_peak(unix_seconds);
    let (hit, miss, output) = match model {
        m if m.starts_with("deepseek-flash") => (0.003, 0.15, 0.60),
        m if m.starts_with("deepseek-v4-pro") => (0.022, 0.66, 1.98),
        _ => return None,
    };
    let factor = if peak { 2.0 } else { 1.0 };
    Some(Prices {
        hit: hit * factor,
        miss: miss * factor,
        output: output * factor,
    })
}

pub fn cost(model: &str, unix_seconds: u64, hit: u64, miss: u64, output: u64) -> Option<f64> {
    let p = prices(model, unix_seconds)?;
    let million = 1_000_000.0;
    Some(
        (hit as f64 * p.hit + miss as f64 * p.miss + output as f64 * p.output) / million,
    )
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-18 00:00:00 UTC, a Wednesday.
    const WED_MIDNIGHT: u64 = 1_794_960_000;

    #[test]
    fn peak_windows_on_weekdays() {
        assert!(is_peak(WED_MIDNIGHT + 2 * 3_600)); // 02:00
        assert!(is_peak(WED_MIDNIGHT + 7 * 3_600)); // 07:00
        assert!(!is_peak(WED_MIDNIGHT + 5 * 3_600)); // 05:00
        assert!(!is_peak(WED_MIDNIGHT + 12 * 3_600)); // 12:00
    }

    #[test]
    fn weekends_are_off_peak() {
        let saturday = WED_MIDNIGHT + 3 * 86_400;
        assert!(!is_peak(saturday + 2 * 3_600));
    }

    #[test]
    fn flash_prices_match_the_table() {
        let off = prices("deepseek-flash", WED_MIDNIGHT + 12 * 3_600).unwrap();
        assert!((off.hit - 0.003).abs() < 1e-9);
        assert!((off.miss - 0.15).abs() < 1e-9);
        let peak = prices("deepseek-flash", WED_MIDNIGHT + 2 * 3_600).unwrap();
        assert!((peak.miss - 0.30).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_has_no_price() {
        assert!(prices("something-else", WED_MIDNIGHT).is_none());
        assert!(cost("something-else", WED_MIDNIGHT, 1, 1, 1).is_none());
    }

    #[test]
    fn cost_adds_the_three_buckets() {
        // 1M of each at off-peak flash: 0.003 + 0.15 + 0.60
        let c = cost("deepseek-flash", WED_MIDNIGHT + 12 * 3_600, 1_000_000, 1_000_000, 1_000_000)
            .unwrap();
        assert!((c - 0.753).abs() < 1e-6, "{c}");
    }
}

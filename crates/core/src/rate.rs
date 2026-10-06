//! Rates and the reteach threshold, in whole numbers only.
//!
//! A rate is kept as two counts (correct, attempts). The shown percentage is
//! floor(100 * correct / attempts), so it never rounds up across a whole-number
//! threshold: 69.5% shows as 69%. "Below the threshold" is the exact test
//! 100 * correct < threshold * attempts, so exactly 70% is not below 70.

/// The threshold used until the teacher sets another one.
pub const DEFAULT_THRESHOLD: u32 = 70;

/// floor(100 * correct / attempts), or None when there are no attempts.
pub fn percent_floor(correct: u32, attempts: u32) -> Option<u32> {
    if attempts == 0 {
        return None;
    }
    Some((u64::from(correct) * 100 / u64::from(attempts)) as u32)
}

/// Whether correct/attempts is strictly below `threshold` percent, or None
/// when there are no attempts (no data, so no suggestion).
pub fn below_threshold(correct: u32, attempts: u32, threshold: u32) -> Option<bool> {
    if attempts == 0 {
        return None;
    }
    Some(u64::from(correct) * 100 < u64::from(threshold) * u64::from(attempts))
}

/// Reads the threshold the teacher typed: a whole number from 0 to 100, with
/// an optional trailing `%`.
pub fn parse_threshold(text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    let digits = trimmed.strip_suffix('%').unwrap_or(trimmed).trim();
    let message = || {
        format!("The threshold must be a whole number from 0 to 100 (you typed \"{trimmed}\").")
    };
    if digits.is_empty() || digits.len() > 3 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(message());
    }
    let value: u32 = digits.parse().map_err(|_| message())?;
    if value > 100 {
        return Err(message());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_floor_hand_values() {
        // worked out by hand: 600/9 = 66.6.., 300/9 = 33.3.., 800/9 = 88.8..,
        // 700/9 = 77.7.., 700/10 = 70, 13900/200 = 69.5, 200/3 = 66.6..
        let cases = [
            ((6, 9), Some(66)),
            ((3, 9), Some(33)),
            ((8, 9), Some(88)),
            ((7, 9), Some(77)),
            ((7, 10), Some(70)),
            ((139, 200), Some(69)),
            ((2, 3), Some(66)),
            ((1, 3), Some(33)),
            ((0, 5), Some(0)),
            ((5, 5), Some(100)),
            ((0, 0), None),
        ];
        for ((c, a), want) in cases {
            assert_eq!(percent_floor(c, a), want, "percent_floor({c}, {a})");
        }
    }

    #[test]
    fn below_threshold_boundary() {
        let cases = [
            ((7, 10, 70), Some(false)), // exactly 70% is not below 70
            ((139, 200, 70), Some(true)), // 69.5% is below 70
            ((69, 100, 70), Some(true)),
            ((6, 9, 70), Some(true)),  // 66.6..%
            ((7, 9, 70), Some(false)), // 77.7..%
            ((7, 9, 80), Some(true)),
            ((0, 9, 0), Some(false)), // nothing is below 0
            ((9, 10, 100), Some(true)),
            ((10, 10, 100), Some(false)),
            ((0, 0, 70), None), // no attempts: no suggestion
        ];
        for ((c, a, t), want) in cases {
            assert_eq!(below_threshold(c, a, t), want, "below_threshold({c}, {a}, {t})");
        }
    }

    #[test]
    fn shown_percent_and_suggestion_never_disagree() {
        for attempts in 1..=40u32 {
            for correct in 0..=attempts {
                let shown = percent_floor(correct, attempts).expect("has attempts");
                for threshold in 0..=100u32 {
                    assert_eq!(
                        below_threshold(correct, attempts, threshold),
                        Some(shown < threshold),
                        "{correct}/{attempts} at {threshold}"
                    );
                }
            }
        }
    }

    #[test]
    fn parse_threshold_accepts_whole_numbers_0_to_100() {
        for (text, want) in [("70", 70), (" 85 ", 85), ("0", 0), ("100", 100), ("70%", 70), ("007", 7)] {
            assert_eq!(parse_threshold(text), Ok(want), "{text:?}");
        }
    }

    #[test]
    fn parse_threshold_rejects_everything_else() {
        for text in ["", "  ", "101", "-5", "7.5", "abc", "1000", "1e2", "%", "70 %%"] {
            let err = parse_threshold(text).expect_err(text);
            assert!(err.contains("whole number from 0 to 100"), "{text:?}: {err}");
        }
    }

    #[test]
    fn default_threshold_is_70() {
        assert_eq!(DEFAULT_THRESHOLD, 70);
    }
}

//! Port of `packages/core/src/duration.ts`.

use std::fmt;

/// Pattern string shared with the policy JSON schema (`pattern` property).
/// Same text as `DURATION_PATTERN` in `duration.ts`.
pub const DURATION_PATTERN: &str = "^[1-9][0-9]*(ms|s|m|h)$";

/// Equivalent of the `RangeError("invalid duration: <input>")` thrown by the
/// TS `parseDuration`. The numeric text is parsed through JS `Number`
/// (`f64`) semantics — including the `2^53` rounding band — before the
/// `u64` result check (documented deviation #8: values past `u64::MAX`
/// are rejected even though TS can return a finite huge `f64`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidDurationError {
    input: String,
}

impl InvalidDurationError {
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for InvalidDurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid duration: {}", self.input)
    }
}

impl std::error::Error for InvalidDurationError {}

fn invalid(input: &str) -> InvalidDurationError {
    InvalidDurationError {
        input: input.to_string(),
    }
}

/// `MULTIPLIERS` suffix table. `"ms"` must be tested before `"s"`.
const SUFFIXES: &[(&str, u64)] = &[("ms", 1), ("s", 1_000), ("m", 60_000), ("h", 3_600_000)];

/// Parse `<non-zero uint><ms|s|m|h>` into milliseconds.
pub fn parse_duration(input: &str) -> Result<u64, InvalidDurationError> {
    let (digits, multiplier) = SUFFIXES
        .iter()
        .find_map(|(suffix, m)| input.strip_suffix(suffix).map(|d| (d, *m)))
        .ok_or_else(|| invalid(input))?;
    let mut bytes = digits.bytes();
    match bytes.next() {
        Some(b'1'..=b'9') => {}
        _ => return Err(invalid(input)),
    }
    if !bytes.all(|b| b.is_ascii_digit()) {
        return Err(invalid(input));
    }
    // TS: `Number(digits) * multiplier` in f64. `digits.parse::<f64>()`
    // reproduces `Number`'s round-to-nearest for the integer text —
    // e.g. "9007199254740993" → 9007199254740992.
    let value: f64 = digits.parse().map_err(|_| invalid(input))?;
    let v = value * multiplier as f64;
    if !v.is_finite() || v > u64::MAX as f64 {
        return Err(invalid(input));
    }
    Ok(v as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_pattern_is_the_shared_source_string() {
        assert_eq!(DURATION_PATTERN, "^[1-9][0-9]*(ms|s|m|h)$");
    }

    #[test]
    fn parses_to_milliseconds() {
        let cases: &[(&str, u64)] = &[
            ("500ms", 500),
            ("1ms", 1),
            ("30s", 30_000),
            ("5m", 300_000),
            ("2h", 7_200_000),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_duration(input).unwrap(), *expected, "{input}");
        }
    }

    #[test]
    fn rejects_invalid_inputs() {
        let cases = [
            "0s", "0ms", "01s", "10", "s", "1.5s", "1d", "-5s", "", "ms", "10ss", " 5s", "5s ",
        ];
        for input in cases {
            let err = parse_duration(input).unwrap_err();
            assert_eq!(err.to_string(), format!("invalid duration: {input}"));
        }
    }

    #[test]
    fn js_number_semantics_round_at_2_to_53() {
        // TS `Number("9007199254740993")` rounds to 2^53; the f64 path
        // must reproduce that rather than reject (probed: 9007199254740992).
        assert_eq!(
            parse_duration("9007199254740993ms").unwrap(),
            9_007_199_254_740_992
        );
        // 2^63-1 rounds to 2^63 in f64 — still ≤ u64::MAX, so accepted.
        assert_eq!(
            parse_duration("9223372036854775807ms").unwrap(),
            9_223_372_036_854_775_808
        );
    }

    #[test]
    fn rejects_overflow() {
        assert!(parse_duration("18446744073709551616h").is_err());
        assert!(parse_duration("18446744073709551615h").is_err());
        // u64::MAX ms rounds to 2^64 in f64 — the saturating cast keeps
        // u64::MAX (documented u64-boundary deviation).
        assert_eq!(parse_duration("18446744073709551615ms").unwrap(), u64::MAX);
        // Multiplied values past u64 are rejected though TS returns a
        // finite f64.
        assert!(parse_duration("5124095576030431000h").is_err());
    }
}

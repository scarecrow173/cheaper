//! JS `String(v)` coercion used by TS error messages
//! (`unsupported $schema: ${String(v)}`, `invalid execution.timeout:
//! ${String(v)}`, `unknown tool in policy: ${String(v)}`).

use serde_json::Value;

/// JS `String(v)` for a `serde_json::Value`. `undefined` cannot be
/// represented in `serde_json`; callers pass [`js_undefined`] for absent
/// values instead.
pub fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        // JS `String(number)` is the canonical number format (no trailing
        // `.0`, JS exponential rules).
        Value::Number(n) => crate::json::number::format_number(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        // `Array.prototype.toString` joins element `String()`s with ",";
        // null/undefined elements yield "".
        Value::Array(a) => a
            .iter()
            .map(|e| match e {
                Value::Null => String::new(),
                other => js_string(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// JS `String(undefined)`.
pub fn js_undefined() -> &'static str {
    "undefined"
}

/// ICU root collation coarse class: whitespace/control sort first,
/// then punctuation/symbols, then digits, then letters. This is the
/// ordering Node/ICU applies at the primary level to the string shapes
/// in scope (`"a_1" < "a0"`, `"_A" < "AA"`, `"gpt_4o" < "gpt4o"`).
fn collation_class(c: char) -> u8 {
    if c.is_whitespace() || c.is_control() {
        0
    } else if c.is_ascii_digit() {
        2
    } else if c.is_alphanumeric() {
        3
    } else {
        1
    }
}

/// DUCET primary order of the ASCII punctuation/symbol class, probed
/// from Node's `localeCompare` (`"_", "-", ",", ";", ... "$"`).
/// Task names and env keys make this ordering reachable (`"a_1"` vs
/// `"a-2"`). Non-ASCII punct/symbols/currency rank after the table —
/// residual divergence (their DUCET weights are not reproduced).
const ASCII_PUNCT_ORDER: &str = "_-,;:!?.'\"()[]{}@*/&#%`^+<=>|~$";

fn punct_rank(c: char) -> u32 {
    match ASCII_PUNCT_ORDER.find(c) {
        Some(i) => i as u32,
        None => 0x1000 + c as u32,
    }
}

/// Approximation of `a.localeCompare(b)` under Node's default (ICU root)
/// collation for the identifier-ish strings in scope (provider ids, env
/// keys, task names): primary compare on the per-char key
/// `(collation_class, char.to_lowercase())` so whitespace/control <
/// punctuation/symbols < digits < letters; then the ICU tertiary rule
/// that a lowercase letter sorts before its uppercase pair
/// (`"a" < "A"`, `"Path" < "PATH"`, `"anthropic" < "OpenAI"`).
///
/// Residual divergence: full ICU collation is not replicated —
/// non-ASCII punctuation/symbols rank by code point after the ASCII
/// DUCET table rather than by their true weights, accents/diacritics
/// (secondary weights), non-ASCII digits (classed as letters), multi-char
/// case folds (e.g. `İ`), and locale-specific reorderings compare
/// differently than in Node.
pub fn locale_compare(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // Primary pass: lexicographic over `(class, lowercase)` per char;
    // a proper prefix sorts first.
    let mut ac = a.chars();
    let mut bc = b.chars();
    let primary = loop {
        match (ac.next(), bc.next()) {
            (None, None) => break Ordering::Equal,
            (None, Some(_)) => break Ordering::Less,
            (Some(_), None) => break Ordering::Greater,
            (Some(ca), Some(cb)) => {
                let class = collation_class(ca);
                let ord = class.cmp(&collation_class(cb)).then_with(|| {
                    if class == 1 {
                        punct_rank(ca).cmp(&punct_rank(cb))
                    } else {
                        ca.to_lowercase().cmp(cb.to_lowercase())
                    }
                });
                if ord != Ordering::Equal {
                    break ord;
                }
            }
        }
    };
    if primary != Ordering::Equal {
        return primary;
    }
    // Tertiary pass: at positions where the chars differ only in case,
    // lowercase wins — the inverse of ASCII byte order for a case pair.
    let mut ac = a.chars();
    let mut bc = b.chars();
    loop {
        match (ac.next(), bc.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca == cb {
                    continue;
                }
                return match (ca.is_lowercase(), cb.is_lowercase()) {
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    // Non-cased or same-case divergence (only reachable via
                    // multi-char lowercase expansions): deterministic order.
                    _ => cb.cmp(&ca),
                };
            }
        }
    }
}

/// JS `<`/`>` string comparison (and default `Array#sort` order):
/// UTF-16 code-unit order. Differs from `str::cmp` (UTF-8 / code-point
/// order) only when an astral-plane char is compared against a BMP char
/// above U+E000 (astral lead surrogates 0xD800–0xDBFF sort below
/// 0xE000–0xFFFF in code-unit order).
pub fn code_unit_compare(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// JS `String.prototype.trim` boundary check. JS strips Unicode
/// `White_Space` plus U+FEFF (ZERO WIDTH NO-BREAK SPACE, retained for
/// compat); Rust's `char::is_whitespace` is Unicode `White_Space` only,
/// so FEFF is the one extra character.
fn is_js_trim_char(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

/// `str.trim` under JS `String.prototype.trim` semantics.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_trim_char)
}

/// JS `{...target, ...value}` own-enumerable-property spread into an
/// insertion-ordered `Map`: objects contribute their entries (existing
/// keys keep position — `IndexMap::insert` matches JS), arrays their
/// index keys, strings their UTF-16 code units as `"0"`, `"1"`, …
/// values, and numbers/booleans/null nothing. Used by the workflow
/// artifacts merge and the restrict-concurrency merge.
pub fn spread_into(
    target: &mut serde_json::Map<String, serde_json::Value>,
    value: &serde_json::Value,
) {
    use serde_json::Value;
    match value {
        Value::Object(m) => {
            for (k, v) in m {
                target.insert(k.clone(), v.clone());
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                target.insert(i.to_string(), v.clone());
            }
        }
        Value::String(s) => {
            for (i, u) in s.encode_utf16().enumerate() {
                target.insert(i.to_string(), Value::String(String::from_utf16_lossy(&[u])));
            }
        }
        _ => {}
    }
}

/// JS `Number.MAX_SAFE_INTEGER`.
pub const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// `Number.MAX_SAFE_INTEGER` as `i64` (exact in both directions).
pub const MAX_SAFE_INTEGER_I64: i64 = 9_007_199_254_740_991;

/// `Number.MAX_SAFE_INTEGER` as `u64` (exact in both directions).
pub const MAX_SAFE_INTEGER_U64: u64 = 9_007_199_254_740_991;

/// `Number.isSafeInteger(v)` → the integer value as `f64` (`None`
/// otherwise). Handles i64/u64 and integral f64 like JS.
pub fn safe_integer(v: &Value) -> Option<f64> {
    let n = v.as_f64()?;
    if n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER {
        Some(n)
    } else {
        None
    }
}

/// Safe non-negative integer as `u64` (`None` for non-integer/negative).
pub fn safe_u64(v: &Value) -> Option<u64> {
    let n = safe_integer(v)?;
    if n < 0.0 {
        return None;
    }
    Some(n as u64)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn string_coercion_matches_js() {
        assert_eq!(js_string(&json!(null)), "null");
        assert_eq!(js_string(&json!(true)), "true");
        assert_eq!(js_string(&json!("x")), "x");
        assert_eq!(js_string(&json!(1.0)), "1");
        assert_eq!(js_string(&json!(1.5)), "1.5");
        assert_eq!(js_string(&json!(1e21)), "1e+21");
        assert_eq!(js_string(&json!([1, null, "x"])), "1,,x");
        assert_eq!(js_string(&json!({"a": 1})), "[object Object]");
        assert_eq!(js_undefined(), "undefined");
    }

    #[test]
    fn locale_compare_matches_node_default_collation() {
        use std::cmp::Ordering;
        let sorted = |v: &[&str]| -> Vec<String> {
            let mut v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
            v.sort_by(|a, b| locale_compare(a, b));
            v
        };
        // ICU: base letters first, then lowercase < uppercase.
        assert_eq!(sorted(&["a", "B", "b", "A"]), ["a", "A", "b", "B"]);
        assert_eq!(sorted(&["Path", "PATH"]), ["Path", "PATH"]);
        assert_eq!(sorted(&["OpenAI", "anthropic"]), ["anthropic", "OpenAI"]);
        assert_eq!(sorted(&["z", "a"]), ["a", "z"]);
        // Pointwise order on differing pairs.
        assert_eq!(locale_compare("a", "A"), Ordering::Less);
        assert_eq!(locale_compare("A", "a"), Ordering::Greater);
        assert_eq!(locale_compare("ab", "aB"), Ordering::Less);
        assert_eq!(locale_compare("aB", "Az"), Ordering::Less);
        assert_eq!(locale_compare("same", "same"), Ordering::Equal);
        assert_eq!(locale_compare("", "a"), Ordering::Less);
        // Node-verified ordering (whitespace/punct < digits < letters,
        // with the ASCII DUCET punct permutation `_` < `-` < `:` < `.` <
        // `[` < `~`):
        assert_eq!(
            sorted(&["_", "-", "0", "a", "A", ".", "~", "z", "1", ":", "["]),
            ["_", "-", ":", ".", "[", "~", "0", "1", "a", "A", "z"]
        );
        // Node outputs: a_1<a0 (-1), _A<AA (-1), gpt_4o<gpt4o (-1),
        // qwen3.5:9b>qwen10:1 (1).
        assert_eq!(locale_compare("a_1", "a0"), Ordering::Less);
        assert_eq!(locale_compare("_A", "AA"), Ordering::Less);
        assert_eq!(locale_compare("gpt_4o", "gpt4o"), Ordering::Less);
        assert_eq!(locale_compare("qwen3.5:9b", "qwen10:1"), Ordering::Greater);
        assert_eq!(locale_compare("a-b", "a0"), Ordering::Less);
        assert_eq!(locale_compare("9", "10"), Ordering::Greater);
        // Intra-punct DUCET order, reachable via task names/env keys:
        // Node gives `a_1` < `a-2` and `path_` < `path-` even though `-`
        // has the smaller byte value.
        assert_eq!(locale_compare("a_1", "a-2"), Ordering::Less);
        assert_eq!(locale_compare("path_", "path-"), Ordering::Less);
        assert_eq!(locale_compare("_A", "-A"), Ordering::Less);
    }

    #[test]
    fn code_unit_compare_uses_utf16_order() {
        use std::cmp::Ordering;
        // BMP-only strings coincide with UTF-8 order.
        assert_eq!(code_unit_compare("abc", "abd"), Ordering::Less);
        assert_eq!(code_unit_compare("a", "ab"), Ordering::Less);
        // Astral char (lead surrogate 0xD800+) sorts below U+E000+ BMP
        // chars in UTF-16 order but above them in UTF-8/code-point order.
        let astral = "\u{10000}";
        let bmp_high = "\u{FFFD}";
        assert_eq!(code_unit_compare(astral, bmp_high), Ordering::Less);
        assert_eq!(astral.cmp(bmp_high), Ordering::Greater);
    }

    #[test]
    fn js_trim_strips_feff_like_js() {
        assert_eq!(js_trim("  x  "), "x");
        assert_eq!(js_trim("\u{FEFF}x\u{FEFF}"), "x");
        assert_eq!(js_trim("\u{FEFF}"), "");
        assert_eq!(js_trim("\u{FEFF}---"), "---");
        assert_eq!(js_trim("\t\n x\r\n"), "x");
    }
}

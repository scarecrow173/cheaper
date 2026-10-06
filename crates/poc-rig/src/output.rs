use serde::Serialize;

/// One-line JSON to stdout; diagnostics go to stderr, never stdout.
pub fn emit(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string(value).expect("envelope serializes")
    );
}

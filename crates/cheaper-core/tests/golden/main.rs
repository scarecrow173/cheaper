//! Golden fixture replay harness: replays `tests/golden/fixtures/*.json`
//! (dumped from `packages/core` by `scripts/dump-core-golden.mjs`)
//! against the Rust port and compares normalized outputs.
//!
//! Normalization rules (per the Phase 1 plan):
//! - schema validation errors compare `instancePath` only;
//! - errno diagnostic values are excluded;
//! - tokenizer identity strings are excluded;
//! - absolute paths are bound through `<ROOT>`/`<OUT>`/`<ABS>`/`<FSROOT>`/
//!   `<BASE>` tokens re-materialized per run.

mod common;
mod deny_paths;
mod dirs;
mod duration;
mod errors;
mod model_classify;
mod model_resolve;
mod number_format;
mod policy_load;
mod policy_merge;
mod policy_process;
mod policy_restrict;
mod policy_schema;
mod resources;
mod token_count;
mod vocabulary;
mod worker_parse;
mod worker_resolve;
mod worker_schema;
mod worker_validate;
mod workspace_paths;

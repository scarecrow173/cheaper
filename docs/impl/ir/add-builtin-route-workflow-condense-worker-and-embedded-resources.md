---
id: "IMPL-034JlxHYMvwfQ6QVTmGtMk"
type: "impl"
status: "completed"
title: "Add builtin route workflow condense worker and embedded resources"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
        note: "builtin route 宣言(cargo-test → routed-exec workflow)"
      - type: "file"
        path: "crates/cheaper-cli/resources/workflows/routed-exec.workflow.json"
        note: "exec + analyze(when condense) routed workflow"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/condense.md"
        note: "condense worker(read-only route log summarizer)"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/condense.input.schema.json"
        note: "logRef/originalTokens/command input schema"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/condense.output.schema.json"
        note: "summary output schema"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/embedded.rs"
        note: "WORKFLOWS_DIR/ROUTES_YAML 追加 + builtin_resources shared list"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_context.rs"
        note: "builtin routed-exec workflow の compile test 追加"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/builtin_workers.rs"
        note: "condense resolve + schema 検証追加"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-builtin-route-resources.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add builtin route workflow condense worker and embedded resources

## Summary

D2 — builtin route resource を embedded resource として追加し、
`extract_builtin` / `embedded_fingerprint` の列挙を単一
`builtin_resources()` list に集約した。`routes.yaml`（cargo-test
route）、`workflows/routed-exec.workflow.json`（exec + analyze
`when` → condense worker）、`workers/condense.md` + input/output
schema を production resource として追加。builtin policy へ grant
は追加しない（no-grant は D1 の revalidation で fail-closed）。

## Context

- task: `docs/tasks/add-builtin-route-resources.md`
- upstream: `docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md` Phase D2
- D1 IR: `docs/impl/ir/add-route-run-delegated-workflow-execution.md`（routed 実行経路が消費する宣言 resource）

## Implementation

### Changes

- `resources/routes.yaml`: `cargo-test` route — match `[cargo, test]`、target `routed-exec.workflow.json`、grant `cargo-test`、tokenThreshold 8192、passthroughEnv `[CI]`。`enabled` key 無し（schema 上 routes は implicit enabled）。
- `resources/workflows/routed-exec.workflow.json`: inputs は D1 の 8 key exact（grant/executable/cwd/runId string、argv/args/execArgs string array、tokenThreshold number ≥0）。`exec` は tool/command `{name:$input.grant,args:$input.execArgs}`、`analyze` は `when: outputTokens > tokenThreshold` で condense worker へ `{logRef,originalTokens,command:$input.argv}`（command は raw string ではなく sanitized argv array — env/prefix を含まない）。outputs.execResult は `{"$from":"exec","path":""}`、outputs.result は `{"$from":"analyze","path":"/summary"}` schema `["string","null"]`（when skip → null を許容）。
- `resources/workers/condense.md` + schemas: tools `[read]` 限定。input schema — logRef runId pattern、originalTokens integer ≥0、command string array、additionalProperties:false。output — summary string minLength1。
- `embedded.rs`: `WORKFLOWS_DIR`（`include_dir!`）+ `ROUTES_YAML`（`include_str!`）を追加し、`builtin_resources()` が `workers/*`・`workflows/*`・`policy.yaml`・`routes.yaml`・`version.json` を path sort 済み `(PathBuf,&[u8])` で返す shared list に。`extract_builtin`・`embedded_fingerprint` が共にこの list を消費（enumerate drift を構造的に排除）。`extract_dir` helper は shared list 化で dead になったため削除。

### Decisions During Implementation

- `result` output schema に `["string","null"]` — engine が `when` skip の worker output を null として返すため。D1 `finish()` が skipped→fallback text を扱う既存経路と整合。
- builtin policy への grant 追加は行わない — no-grant の cargo-test route は D1 revalidation で grant 不在 → fail する fail-closed 挙動のまま。user が自ら grant を承認するまで route は動かない。
- E2E（実 `cargo test` route 実行）は D2 範囲外 — libtest の positional filter 制約（`test` + testname の2 filter）により grant child 経路が unsupported。compiler test（`workflow_context::tests::builtin_routed_exec_workflow_compiles`）が D2 Done When をカバー。E で native stub semantics を使う。

## Related Experiments

### Adopted

### Rejected

## Validation

- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib embedded::` → 4/4 pass(`tmp/handoff-d2/embedded.txt` — extraction path 集合・byte 一致・fingerprint 攪乱感応)。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib workflow_context::` → 1/1 pass(`tmp/handoff-d2/context.txt` — builtin routed-exec workflow が 8 inputs で compile 成功、`when` の `/details/outputTokens`・`$input:tokenThreshold` binding を assert)。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration builtin_workers::` → 2/2 pass(`tmp/handoff-d2/builtin-workers.txt` — condense resolve + input/output schema 構造検証)。
- `mise exec -- cargo fmt --all -- --check` → clean(`tmp/handoff-d2/fmt.txt`)。
- `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` → warning 0(`tmp/handoff-d2/clippy.txt` — `extract_dir` dead code 検出 → 削除で解消)。
- impl-doc audit: `audit_impl_record.js --dir docs/impl/ir` → No findings。

## Risks

- `when` skip の `result` null → D1 `finish()` の clean path が `outputs.result` nonempty string を要求するため skip 時は fallback text へ。既存 `skipped_analyze_keeps_fallback_text` integration test と同型で挙動確認済み。

## Follow-ups

- D4: `route run --payload` CLI arm dispatch。
- E: worker observable read evidence（fake provider の `ToolResult` error が outcome に反映されない seam 設計上、analyze 自身の read 成功は別 seam で検証する。D1 IR review 補正と同じ区分を維持）。

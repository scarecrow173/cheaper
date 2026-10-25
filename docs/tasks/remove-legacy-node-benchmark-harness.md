---
id: "TASK-6DBO0a5MUlapTYav2YJtS0"
type: "task"
status: "todo"
title: "Remove legacy Node benchmark harness — benchmarks/*.mjs, lib, fixtures, test suite"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
  changes:
    added: []
    modified: []
    deleted:
      - type: "file"
        path: "benchmarks/run.mjs"
      - type: "file"
        path: "benchmarks/lib/cli.mjs"
      - type: "file"
        path: "benchmarks/lib/run-scenario.mjs"
      - type: "file"
        path: "benchmarks/lib/measurements.mjs"
      - type: "file"
        path: "benchmarks/lib/result-schema.mjs"
      - type: "file"
        path: "benchmarks/lib/scenarios.mjs"
      - type: "file"
        path: "benchmarks/lib/aggregate.mjs"
      - type: "file"
        path: "benchmarks/fixtures/fake-worker-stream.mjs"
      - type: "file"
        path: "benchmarks/test/aggregate.test.mjs"
      - type: "file"
        path: "benchmarks/test/measurements.test.mjs"
      - type: "file"
        path: "benchmarks/test/result-schema.test.mjs"
      - type: "file"
        path: "benchmarks/test/scenario-manifest.test.mjs"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "docs/tasks/verify-bench-smoke-run-on-real-binary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cheaper-bench-harness-contract-issue-183.md"
  references:
    - "docs/designs/benchmark-harness-redesign-issue-183.md"
  defers: []
  deferred-by: []
---

# Remove legacy Node benchmark harness

## Work

T1-T5 が green(新 Rust harness が実 binary で smoke run を完走)した
**後置**で、旧 Node harness の全 `.mjs` file を削除する。
**ordering constraint**: `node --test benchmarks/test/` の pipeline 行は
T2 で除去済みであること(manifest v2 flip と同一変更) — 行削除 →
file 削除の順を崩さない(逆順だと check が存在しない dir を
`node --test` して壊れる)。T7(全実測)とは並列可 — file 面が
交差しない。

1. 削除対象(12 file):
   - `benchmarks/run.mjs`(74 行 — CLI entry)
   - `benchmarks/lib/cli.mjs`(165)
   - `benchmarks/lib/run-scenario.mjs`(461)
   - `benchmarks/lib/measurements.mjs`(92)
   - `benchmarks/lib/result-schema.mjs`(117)
   - `benchmarks/lib/scenarios.mjs`(66)
   - `benchmarks/lib/aggregate.mjs`(175)
   - `benchmarks/fixtures/fake-worker-stream.mjs`(102 — RunFixture DSL
     の JS 平行実装。Rust 化で drift 面が消える)
   - `benchmarks/test/aggregate.test.mjs`・`measurements.test.mjs`・
     `result-schema.test.mjs`・`scenario-manifest.test.mjs`
     (計 312 — coverage は `crates/cheaper-bench/tests/` 4 file へ
     移植済み)
   - 削除後に空になった `benchmarks/lib/`・`benchmarks/fixtures/`・
     `benchmarks/test/` の空 dir も除去する(git は空 dir を保持
     しない — file 削除で自然消滅するが、残存 `.gitkeep` 等があれば
     確認)。

2. 維持するもの(削除禁止):
   - `benchmarks/scenarios/`(scenarios.json = manifest v2・
     saved-workflow.json)
   - `benchmarks/results/`(v1 JSON 不変・README・新 v2 artifact)
   - `benchmarks/README.md`(T4 で Rust harness 版へ書換済み)

3. 残存参照の grep 確認: 削除後に `benchmarks/` 配下・`mise.toml`・
   `biome.jsonc`・`.pre-commit-config.yaml`・`AGENTS.md`・
   `CONTRIBUTING.md`・`docs/` に `.mjs` harness への生参照が残らない
   ことを確認する(`.pre-commit-config.yaml` の biome mirror files
   pattern は残存 `benchmarks/**/*.json` に合致 — 変更不要)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md (T6 slice — ordering constraint)
- docs/designs/benchmark-harness-redesign-issue-183.md (§6 doc-sync 削除行)

## Dependencies

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md
- docs/tasks/verify-bench-smoke-run-on-real-binary.md (**T1-T5 green が
  前提の ordering constraint** — smoke run が新 harness の動作を証明
  してから旧 harness を削除する)

## Affected Files

- `benchmarks/run.mjs`・`benchmarks/lib/*.mjs`(6)・
  `benchmarks/fixtures/fake-worker-stream.mjs`・
  `benchmarks/test/*.mjs`(4)を削除(計 12 file)

対象外: `benchmarks/scenarios/`・`benchmarks/results/`・
`benchmarks/README.md`(全て維持)、product crate、`mise.toml`
(T2 で確定済み)。

## Done When

- [ ] 上記 12 file が削除され、空 dir(`lib`/`fixtures`/`test`)が
      残っていない
- [ ] `benchmarks/` 配下に tracked `.mjs` file が 1 件も残っていない
      (`git ls-files` で確認)
- [ ] `git grep` で `{node}`・`process.execPath`・`node:fs`・
      `environment.node` が `benchmarks/`(`benchmarks/results/`
      除外 — immutable v1 artifact が旧計測 command を literal で
      含むため到達不能)・`crates/cheaper-bench/` に存在しない
- [ ] `mise.toml`・`biome.jsonc`・`.pre-commit-config.yaml`・doc 群に
      `.mjs` harness への生参照が残っていない
- [ ] `benchmarks/results/` の v1 JSON に diff が無い(immutable)
- [ ] `mise run check` が全 pass する(pipeline に node --test
      benchmarks/test/ が無い状態で)

## Verification

- [ ] `mise run check` exits 0
- [ ] `git ls-files -- 'benchmarks/**/*.mjs'` が 0 件(content grep は
      `benchmarks/results/README.md` 等の prose 言及に hit するため、
      file 削除の証明は tracked file 列挙で行う)
- [ ] `git grep -n '{node}\|process\.execPath\|node:fs\|environment\.node' -- benchmarks/ ':(exclude)benchmarks/results/**' crates/cheaper-bench/` が 0 件(`benchmarks/results/` の immutable v1 artifact は `node:fs` 等を含む旧計測記録のため除外)
- [ ] `git grep -n 'benchmarks/test\|benchmarks/lib\|fake-worker-stream' -- mise.toml biome.jsonc .pre-commit-config.yaml AGENTS.md CONTRIBUTING.md docs/` が 0 件(または意図的な言及のみ — `benchmarks/test/` の history 記述等は除く)
- [ ] `git diff -- benchmarks/results/` が v1 JSON に差分を持たない
- [ ] Manual: `biome check` exits 0(dead glob 除去後・`.mjs` 対象消滅で lint が変質しないこと — supplement)

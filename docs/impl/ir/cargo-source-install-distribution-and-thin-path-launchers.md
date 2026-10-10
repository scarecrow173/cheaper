---
id: "IMPL-03426dknEI5DskvoOZUHs"
type: "impl"
status: "completed"
title: "Cargo source install distribution and thin PATH launchers"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/85"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/thin-launchers-rename-rewrite-and-hooks-json-references.md"
    - "../../tasks/slim-payload-acquisition-machinery-removal-and-check-rewiring.md"
    - "../../tasks/docs-and-cheaper-usage-skill-rewrite-for-cargo-source-install.md"
    - "../../tasks/upstream-doc-reconciliation-and-final-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../designs/cargo-source-install-distribution-and-thin-path-launchers.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/cargo-source-install-distribution-and-thin-path-launchers.md"
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

# Cargo source install distribution and thin PATH launchers

## Summary

Issue #85。Release binary acquisition を廃止し、APM consumer の acquisition を
`cargo install --git https://github.com/scarecrow173/cheaper.git --locked cheaper-cli`
に置き換えた。hook launcher は PATH 解決の薄い `cheaper.sh` / `cheaper.ps1`、
APM payload は binary-free（`cli/` 廃止）に slim 化した。

## Context

承認済み design（DESIGN-034Dmk2RqH0fMzF3gLrR0T）と plan の 4 task を実施。
worker 定義・policy・version は既に Rust binary の embedded resource 化済み
であり、`cli/` payload は旧 acquisition/sibling モデル専用だったため、
launchers と `hooks.json` のみを残す構成が最小表現になる。

## Implementation

### Changes

- `packages/apm/.apm/hooks/cheaper` → `cheaper.sh` へ rename（git mode
  `100755` 維持）。`command -v cheaper` で PATH 解決し、argv/stdin/
  stdout/exit code を passthrough。CLI 不在時は stderr に install/PATH
  guidance を出して exit 1（stdout 無し）。
- `cheaper.ps1` を `Get-Command cheaper, cheaper.exe -CommandType Application`
  + null Source guard + `ProcessStartInfo` 起動へ書き換え（PS5.1・BOM・
  CRLF 維持、argv 欠落 workaround 保持）。
- `hooks.json` の POSIX command を `sh ./cheaper.sh`（exec bit 非依存）に
  統一。fail-open suffix（`|| exit 0` 等）は全項目で維持。
- `cli/` payload 全体と acquisition 機構（bundle-cli・assemble・checksum・
  binary-size・release smoke の各 script/test、3 workflow、fixture.cs）を削除。
- `check-apm-payload.mjs` を slim contract 化（extension allowlist・PE `MZ`/
  ELF magic 検出・exec 許可は `.sh` のみ・BOM/CRLF・shebang/LF・index mode
  `100755`・hooks.json 参照検査・bare `./cheaper` 拒否）。
- `check-lockfile.mjs` 新設（`cargo metadata --locked` 大 stdout を discard +
  `git diff --exit-code -- Cargo.lock`）。`mise run check` で cargo
  build/test 後・payload check 前に配置。
- `check-version.mjs` を Cargo.toml ↔ `apm.yml` 2 宣言比較へ簡素化。
  `set-version.mjs` から `version.json` 再生成を除去。
  `release-artifacts.mjs` は version helper のみ残す（`cargoBinary` 削除、
  `nodeFreePath` は passthrough test が利用するため保持）。
- `apm-payload.test.mjs`・`apm-install-targets.test.mjs` を slim contract
  （launcher 3 file・`cli/` 不存在）へ、`launcher-passthrough.test.mjs` 新設
  （argv/stdin/stdout/exit・missing CLI diagnostics、`nodeFreePath` で
  制御 PATH。Windows では `PATHEXT` 継承 + `where sh` の dir を PATH 追加）。
- `distribution.md`・README・`cheaper-usage` SKILL の acquisition 記述を
  cargo install モデルへ書き換え。旧 design/spec/ADR/test-spec を supersede
  または amended clause 付きで mark、index を同期。

### Decisions During Implementation

- `Get-Command` が function/alias を拾い `.Source` 空で誤動作し得るため
  `-CommandType Application` + null guard（design review 指摘）。
- `.cmd` は `-CommandType Application` で解決できる（当初の PATHEXT 欠落が
  原因で C# fixture は不要と判明）。Windows test では `cheaper.exe`/`cheaper`
  両名を `.cmd` stub で検証。
- POSIX launcher を `sh ./cheaper.sh` 経由にしたため deploy 先の exec bit に
  依存しないが、payload contract として index mode `100755` は別途 assert。
- `cargo metadata` の stdout が Node の default buffer を超え ENOBUFS になる
  ため stdout は捨てて status のみ評価。
- Git Bash の `sh.exe` dir を fake PATH に加えることで POSIX test を Windows
  host でも実実行（`sh` 不在なら skip 維持）。

## Related Experiments

### Adopted

- 旧 test の `launcher-fixture.cs` + csc コンパイル経路は `.cmd` stub で
  代替可能と判明し fixture を廃止。

### Rejected

- `cli/` の一部（workers/policy）を APM payload に残す案 — embedded resource
  化で不要であり payload を増やすだけなので不採用。

## Validation

- `mise run check` 全 pass: biome / fmt / clippy / version / doc-id /
  `cargo test --workspace` / node test 30 pass / benchmark 12 pass /
  lockfile check / APM slim payload check。
- launcher passthrough 4 test pass（sh/ps1 × passthrough/missing）。
- Windows 実機 smoke: `sh ./cheaper.sh` で argv/stdin/exit 7 伝播、missing
  CLI で stderr guidance + exit 1 を確認。`powershell -File ./cheaper.ps1`
  も同等。
- local `cargo install --git ... --locked cheaper-cli` v1.0.0 install 成功、
  `C:\Users\akiyoshi\.cargo\bin\cheaper.exe --version` → `1.0.0`、
  `models` envelope 応答、両 launcher 経由でも `1.0.0`。
- APM 0.28.0 temp consumer deploy 実測: `cheaper.sh` mode 755・`cheaper.ps1`
  BOM 保存・生成 `.windsurf/hooks.json` が `sh .../cheaper.sh` + fail-open
  維持。旧 `cli/`・extensionless `cheaper` は reinstall で prune されない
  ことを確認し、upgrade cleanup 手順を `distribution.md` に記録。

## Risks

- Rust toolchain ≥1.95 が consumer 前提要件（rig 0.43 要求）。offline は
  toolchain + cargo cache の事前導入が必要（distribution.md に記載）。
- APM reinstall が stale file を prune しないのは APM 側仕様で、hook は
  `sh ./cheaper.sh` のみ参照するため実害はない。

## Follow-ups

- #87 chain で記録済みの registry/virtual-ref dep name 実測は本 chain で
  追加実測不要（path dep・virtual ref とも `<dep-name>` 規則確認済み）。

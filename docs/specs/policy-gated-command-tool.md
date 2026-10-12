---
id: "SPEC-034EMbEGBOWM4CAM6YSWm6"
type: "spec"
status: "approved"
title: "policy-gated dynamic command を canonical tool として提供する"
created: "2026-10-12"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/101"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: ["./managed-process-canonical-tool.md", "./managed-process-lifecycle-hardening.md"]
  refines: []
  refined-by: []
  derives-from: ["../discovery/policy-gated-command-tool.md"]
  derived-by: []
  verifies: []
  verified-by: []
  references: ["../guides/managed-process.md", "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"]
  defers: []
  deferred-by: []
---

# policy-gated dynamic command を canonical tool として提供する

## What Should Be Built

canonical tool vocabulary に新 tool `command` を追加する。`command` は trusted layer（builtin policy + user-scope `<userConfigDir>/policy.yaml`）で宣言された grant が許可する program に対し、動的な argv を付けて実行する managed process tool である。

`command` は Worker（tools 宣言 ∩ tools.allow ∩ command grant の三重ゲート）と Workflow の `kind: tool` job（tools.allow admission check + nonempty-grants fail-closed registration gate の二重ゲート）の両方から、同一の canonical 実装を通じて利用できる。既存 `process` tool の contract と挙動は不変とする。

## Why Now

既存の `process` tool は `policy.yaml > process.tasks` に事前登録された named task しか実行できず、固定 argv に限定される。git / build / test のような動的 argv を持つコマンドを安価な worker へ委譲する手段がなく、親 Coding Agent は高コスト model で自ら実行するか、任意 shell を開放するしかない。trusted grant の範囲内でのみ動的コマンド実行を許可する `command` tool でこの gap を埋める。

## Users and Value

- 親 Coding Agent: git / build / test 等の動的コマンドを、任意 shell を開放せず安価な worker へ委譲でき、frontier model の context とコストを節約できる。
- trusted user: builtin + user-scope の trusted layer で明示 grant した program と argv pattern のみを実行対象として制御でき、project repo や model が権限を拡大できない。

## Scope

### In Scope

- canonical tool vocabulary への `command` 追加と、Worker / Workflow `kind: tool` job 両経路からの同一実装による実行。
- input schema（grant selector + args 配列、例: `{name, args: string[]}`）と、`process` と同型の structured result（grant id、argv の safe representation、exitCode、state、bounded stdout/stderr、truncation info、duration）。
- trusted layer（builtin policy + user-scope `<userConfigDir>/policy.yaml`）でのみ宣言可能な grant。builtin policy は `command.grants: {}`（空）を同梱し、user-scope が実質の authoring layer となる。project partial schema は `command` 以下の grant authoring key を全て除外し、これらを含む project policy は load 時に schema validation で拒否される。
- executable 解決 + native-magic/shell-shim 拒否、per-token argv pattern match、shell metachar 拒否、cwd normalize + denylist + workspace boundary check、sanitized env という実行前検証。
- 既存 managed-process lifecycle の再利用（spawn / timeout / killGrace / abort / process-tree cleanup / bounded combined stdout+stderr / heartbeat / concurrency cap）。
- global budget、resource scheduling（副作用未知のため保守的な Exclusive resource access）、usage/savings との統合。
- trust / sandbox 限界の文書化。

### Out of Scope

- unrestricted shell、interactive TTY、daemon / 常駐 process。
- OS sandbox 無しでの containment / read-only 保証（declared trust boundary を超える副作用の抑止）。
- tool-operator worker 本体（issue #102 で扱う）。
- artifact-store への実行 log 永続化。
- `process` tool の contract / behavior 変更。project policy からの `process.tasks` extend 許可は現行のまま維持し、`command` のより厳しい境界との divergence として文書化する。

## Requirements

- `command` は default deny。grant が一つも無い環境では tool registration が fail closed（POLICY_VIOLATION）し、tools.allow に `command` を含む worker/workflow でも到達できない。
- input は grant selector + args 配列（例: `{name, args: string[]}`）。model は executable / cwd / env を直接供給しない。
- grant は trusted layer（builtin policy + user-scope `<userConfigDir>/policy.yaml`）でのみ宣言可能とする。builtin policy は `command.grants: {}`（空）を同梱し、user-scope が実質の authoring layer となる。project partial schema は `command` 以下の grant authoring key を全て除外し、これらを含む project policy は schema validation で fail して load 全体が `InvalidRuntimePolicy` で拒否される（per-call ではなく load 時の fail-fast）。project 側の narrowing field は本 scope では提供しない — 将来の明示的な workspace trust 設計に委ねる。
- grant の `executable` は policy load 時に canonical path へ解決して固定する。bare name は load 時点の環境 PATH に対して一度だけ解決し、この provenance（ambient PATH への信頼）は user-facing docs に明記する。実行には native-magic check（MZ / ELF / Mach-O）と shell / shim 拒否を通過しなければならない。さらに shell / shim / interpreter 起動による迂回を既定で禁止し、python / perl / ruby / node 等の interpreter executable は拒否対象とする（issue #101 の「任意の interpreter 起動による迂回は既定で禁止」に一致）。process task で使う denial boundary（`shell_executable_re` + `script_ext_re` + native magic + bare `node` reject）を baseline とし、interpreter executable の網羅性は design で検証して `command` 向けの deny set を必要に応じて拡張する。
- argv は join した文字列ではなく token 配列のまま、grant の argv pattern に対して per-token で match する。shell metachar を含む token は match 前に reject する。
- cwd の default は workspace root。指定 cwd は normalize 後に denylist と workspace boundary check を通す。この検証は宣言された cwd のみに適用し、argv 内の path operand には filesystem 検証を適用しない — argv に対する filesystem 面の唯一の制約は grant の argv pattern である（shell metachar 拒否は filesystem 検証とは別の、match 前の独立したチェック）。
- `command` の子 process は OS sandbox 無しで動作する。`filesystem.denyPaths`・workspace boundary・worker の writeScope は `command` の子 process の副作用を bound しない（grant shape には inputs/outputs が無く、writeScope coverage check は `command` では vacuous）。これらの trust / sandbox 限界は user-facing docs に必ず明記する。
- writeScope を宣言した worker に対して `command` を fail-closed で reject するかは Deferred Design Concern とする。
- 子 process の env は sanitized minimal env + grant-declared の key/value のみ。親 process の ambient credential は継承しない。
- spawn / timeout / killGrace / abort / process-tree cleanup / bounded combined stdout+stderr / heartbeat / concurrency cap は既存 managed-process executor を再利用する。`command` の子 process は `process` と同じ managed-subprocess tracking（`managed.track()` + `drain()` / `wait_idle`）に載せ、runtime shutdown が spawn 済み子 process を待つようにする。
- Workflow `kind: tool` job に対する gate は tools.allow admission check と nonempty-grants fail-closed registration gate の二つとする。worker の tools 宣言 gate は worker 経路のみの gate であり、workflow job には適用しない。
- 副作用が未知のため、resource scheduling では保守的に workspace root claim に対する Exclusive resource access を宣言する（catalog の `resources()` arm と `process_operation_declaration` に相当する operation declaration で宣言する）。
- `meta.changes` は実測した write/edit change のみを報告し、未観測の child side effect を audited として主張しない。`savings.directlyComparable` は `command` の run では常に false とする（child mutation が未観測のため estimated-savings 母数に参加させない）。budget / tool-call accounting の既存契約は不変。

## Acceptance Criteria

- [ ] AC1: 既存 `process` tool の contract と test が変更なく pass する。
- [ ] AC2: `command` は default-disabled であり、grant を持たない Worker / Workflow からは tool 自体に到達できない（tools.allow のみでは不十分）。
- [ ] AC3: trusted layer（builtin / user-scope）の grant が許可する dynamic executable + argv が、Worker と Workflow `kind: tool` job の両経路から同一 canonical 実装で実行できる。
- [ ] AC4: forbidden executable / argv（interpreter executable を含む）、shell injection（metachar を含む token）、cwd escape が実行前に reject され、grant authoring key を含む project policy が load 時に schema validation で拒否される test が通る。
- [ ] AC5: timeout / abort / nonzero-exit / output-cap / process-tree cleanup の test が Windows / POSIX の差分を含めて検証される。
- [ ] AC6: global budget、resource scheduling、usage/savings と統合され、未観測の child mutation が audited change として報告されない。
- [ ] AC7: trust / sandbox 限界が guide に文書化され、`cargo fmt` / `clippy` / `cargo test` が pass する。

## Deferred Design Concerns

- 信頼境界は ADR `../adr/command-grant-trust-boundary-and-managed-process-reuse.md` で確定済み: pure Option A（project partial schema が `command` 以下の grant authoring key を全て除外し、含む project policy は load 時に `InvalidRuntimePolicy` で拒否）。project 側 narrowing field は本 scope では提供せず、将来の明示的 workspace trust 設計に defer する。
- grant 宣言 shape の最終形（`{executable, argv?, cwd?, env?, timeout?, killGrace?, maxOutputBytes?}` の各 override 範囲の配分）は design phase で確定する。
- argv pattern の match 言語（literal / glob / prefix 等の semantics）は design phase で確定する。
- writeScope を宣言した worker に対して `command` を fail-closed で reject するかは design phase で確定する。
- grant executable に bare name を許すか絶対 path を要求するかは design phase で確定する。
- `command` が `process` の pool cap（`OperationPool`）を共有するか独自の cap を持つかは design phase で確定する。
- project policy からの `process.tasks` extend 権限の剥奪は本 spec の範囲外とし、`command` との divergence として文書化する方針を維持する（変更する場合は別 spec で扱う）。

## Sources and Relations

- source: GitHub issue #101（`command` tool の要求と acceptance criteria の正本）。
- derives-from: `../discovery/policy-gated-command-tool.md`（DISC-034EM8dPyCYPdx7fqHlaBU）。touch point 調査、reusable machinery、信頼境界 Option A/B/C を内包する。
- related: `./managed-process-canonical-tool.md`（`process` tool の contract 正本）、`./managed-process-lifecycle-hardening.md`（再利用する lifecycle 保証）。
- references: `../guides/managed-process.md`（managed process の認可・limit 文書）、`../adr/command-grant-trust-boundary-and-managed-process-reuse.md`（grant 信頼境界と managed-process 再利用の確定済み decision）。

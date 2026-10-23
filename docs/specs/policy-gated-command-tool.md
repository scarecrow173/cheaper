---
id: "SPEC-034EMbEGBOWM4CAM6YSWm6"
type: "spec"
status: "approved"
title: "policy-gated dynamic command を canonical tool として提供する"
created: "2026-10-12"
updated: "2026-10-14"
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
  related: ["./managed-process-canonical-tool.md", "./managed-process-lifecycle-hardening.md", "./invocation-scoped-command-grants.md", "./pretooluse-route-resolver-and-workflow-delegation.md", "../adr/command-grant-trust-boundary-and-managed-process-reuse.md", "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md", "../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md", "../discovery/tool-operator-worker-built-in.md", "../discovery/invocation-scoped-command-grants.md", "../designs/tool-operator-built-in-worker.md", "./documentation-honesty-sweep-and-breaking-change-migration-guide.md", "../adr/documentation-honesty-sweep-and-migration-guide.md", "../guides/migration.md", "../designs/documentation-honesty-sweep-and-migration-guide.md"]
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
- executable 解決（spawn viability: realpath・is_file・win32 `.exe`/`.com`・X_OK — 内容審査なし）、per-token argv pattern match、cwd normalize + denylist + workspace boundary check、sanitized env という実行前検証。executable/argv の内容審査は行わない（issue #163 — 内容審査除去）。opt-in deny は `security.execDenyPatterns`（deterministic executable deny-list・既定 `[]`）宣言時のみ（issue #164）。
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
- grant の `executable` は policy load 時に canonical path へ解決して固定する。bare name は load 時点の環境 PATH に対して一度だけ解決し、この provenance（ambient PATH への信頼）は user-facing docs に明記する。ここでの pin は tool runtime 構築の boundary（scope ごとの `create_tool_runtime`）で行われ、「一度だけ」は pin 済み path がその runtime の生存期間中は固定されることを意味し、spawn 時には pin 済み path への再検証を伴う。executable の内容・名は審査しない — opt-in `security.execDenyPatterns` 宣言時のみ deterministic executable deny-list が適用される（issue #163/#164 — 旧 mandatory の native-magic/shell-shim/interpreter/bare-`node` deny は opt-in 前提へ降格）。
- argv は join した文字列ではなく token 配列のまま、grant の argv pattern に対して per-token で match する。call-time の内容審査は行わない — metachar を含む token も pattern 適合なら executable の literal argv として渡る（spawn は shell を介さない）。token 長 ≤4096・args 個数 ≤256 は input schema 層の bound で、超過は `INVALID_INPUT`（issue #163 — metachar 拒否除去）。
- cwd の default は workspace root。指定 cwd は normalize 後に denylist と workspace boundary check を通す。この検証は宣言された cwd のみに適用し、argv 内の path operand には filesystem 検証を適用しない — argv に対する filesystem 面の唯一の制約は grant の argv pattern である。
- `command` の子 process は OS sandbox 無しで動作する。`filesystem.denyPaths`・workspace boundary・worker の writeScope は `command` の子 process の副作用を bound しない（grant shape には inputs/outputs が無く、writeScope coverage check は `command` では vacuous）。これらの trust / sandbox 限界は user-facing docs に必ず明記する。
- writeScope を宣言した worker が `command`（`!fixed` grant surface）を expose すると scope 生成が `POLICY_VIOLATION`（"command is incompatible with worker writeScope" — `registry.rs:1167`）で fail closed する（design phase で確定・実装済み — workers.md:54-67 と整合。issue #169）。`process` task（`fixed` grant）側は `outputs ⊆ writeScope` の coverage tri-state が適用される（`outputs` 宣言（nonempty）は coverage 必須・`outputs: []` は coverage trivially 成立・`outputs` 未宣言は coverage 証明不能で `POLICY_VIOLATION` — workers.md:61-67 の規約）。
- 子 process の env は sanitized minimal env + grant-declared の key/value のみ。親 process の ambient credential は継承しない。
- spawn / timeout / killGrace / abort / process-tree cleanup / bounded combined stdout+stderr / heartbeat / concurrency cap は既存 managed-process executor を再利用する。`command` の子 process は `process` と同じ managed-subprocess tracking（`managed.track()` + `drain()` / `wait_idle`）に載せ、runtime shutdown が spawn 済み子 process を待つようにする。
- Workflow `kind: tool` job に対する gate は tools.allow admission check と nonempty-grants fail-closed registration gate の二つとする。worker の tools 宣言 gate は worker 経路のみの gate であり、workflow job には適用しない。
- 副作用が未知のため、resource scheduling では保守的に workspace root claim に対する Exclusive resource access を宣言する（catalog の `resources()` arm と `process_operation_declaration` に相当する operation declaration で宣言する）。
- `meta.changes` は実測した write/edit change のみを報告し、未観測の child side effect を audited として主張しない。`savings.directlyComparable` は `command` の run では常に false とする（child mutation が未観測のため estimated-savings 母数に参加させない）。budget / tool-call accounting の既存契約は不変。

## Acceptance Criteria

- [ ] AC1: 既存 `process` tool の contract と test が変更なく pass する。
- [ ] AC2: `command` は default-disabled であり、grant を持たない Worker / Workflow からは tool 自体に到達できない（tools.allow のみでは不十分）。
- [ ] AC3: trusted layer（builtin / user-scope）の grant が許可する dynamic executable + argv が、Worker と Workflow `kind: tool` job の両経路から同一 canonical 実装で実行できる。
- [ ] AC4: argv pattern 不一致・cwd escape・grant authoring key を含む project policy が load 時に schema validation で拒否される test が通る。opt-in `security.execDenyPatterns` 宣言時のみ deny 該当 executable が拒否される（issue #163/#164 — 旧 mandatory の forbidden executable/shell injection 拒否 AC は opt-in 前提へ降格）。
- [ ] AC5: timeout / abort / nonzero-exit / output-cap / process-tree cleanup の test が Windows / POSIX の差分を含めて検証される。
- [ ] AC6: global budget、resource scheduling、usage/savings と統合され、未観測の child mutation が audited change として報告されない。
- [ ] AC7: trust / sandbox 限界が guide に文書化され、`cargo fmt` / `clippy` / `cargo test` が pass する。

## Deferred Design Concerns

- 信頼境界は ADR `../adr/command-grant-trust-boundary-and-managed-process-reuse.md` で確定済み: pure Option A（project partial schema が `command` 以下の grant authoring key を全て除外し、含む project policy は load 時に `InvalidRuntimePolicy` で拒否）。project 側 narrowing field は本 scope では提供せず、将来の明示的 workspace trust 設計に defer する。
- grant 宣言 shape の最終形（`{executable, argv?, cwd?, env?, timeout?, killGrace?, maxOutputBytes?}` の各 override 範囲の配分）は design phase で確定する。
- argv pattern の match 言語（literal / glob / prefix 等の semantics）は design phase で確定する。
- writeScope を宣言した worker に対する `command` の扱いは確定済み: fail-closed が採用・実装されている（`registry.rs:1167` — writeScope 宣言 worker が `command`（`!fixed` grant surface）を expose すると scope 生成が `POLICY_VIOLATION`。guide 反映済み workers.md:54-67。issue #169）。
- grant executable に bare name を許すか絶対 path を要求するかは design phase で確定する。
- `command` が `process` の pool cap（`OperationPool`）を共有するか独自の cap を持つかは design phase で確定する。
- project policy からの `process.tasks` extend 権限の剥奪は本 spec の範囲外とし、`command` との divergence として文書化する方針を維持する（変更する場合は別 spec で扱う）。

## Sources and Relations

- source: GitHub issue #101（`command` tool の要求と acceptance criteria の正本）。
- derives-from: `../discovery/policy-gated-command-tool.md`（DISC-034EM8dPyCYPdx7fqHlaBU）。touch point 調査、reusable machinery、信頼境界 Option A/B/C を内包する。
- related: `./managed-process-canonical-tool.md`（`process` tool の contract 正本）、`./managed-process-lifecycle-hardening.md`（再利用する lifecycle 保証）、`../adr/command-grant-trust-boundary-and-managed-process-reuse.md`（grant 信頼境界と managed-process 再利用の decision）。
- references: `../guides/managed-process.md`（managed process の認可・limit 文書）、`../adr/command-grant-trust-boundary-and-managed-process-reuse.md`（grant 信頼境界と managed-process 再利用の確定済み decision）。

## Refine 注記（issue #169）

権限モデル再設計（#151 分解 #163–#168）により、本 doc の要件本文は in-place で現行の真実へ訂正済み（spec は living contract — 訂正行に issue 注釈を付す方式）。

- `:60`・`:78` — executable 解決の実行前検証から native-magic/shell-shim/interpreter/bare-`node` の mandatory deny 要件を除去 → 内容審査なし・opt-in `security.execDenyPatterns` のみ（#163/#164）。
- `:79-:80` — call-time の shell metachar 拒否を除去 → literal argv 透過 + input schema bound（#163）。
- `:94` AC4 — 旧 mandatory 拒否 AC を opt-in 前提へ降格（#163/#164）。
- `:82`・`:104` — writeScope × `command` の Deferred 記述を「確定済み」（`registry.rs:1167` fail-closed）へ訂正（#169）。

trusted-layer grant 境界（Option A）・pin 契約・argv pattern・cwd/env 境界・lifecycle 要件は維持。

governing: [ADR-034L6ihibvbjmSnKmSffWH](../adr/documentation-honesty-sweep-and-migration-guide.md)（canonical 文言・訂正方式）・[SPEC-034L6i3XmfCaWRHAiw8TTT](./documentation-honesty-sweep-and-breaking-change-migration-guide.md) R5・[migration guide](../guides/migration.md)

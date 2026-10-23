---
id: "ADR-034KI3r2xdthh6rKDFY517"
type: "adr"
status: "proposed"
title: "exec-contract cleanup — 内容審査を mandatory 層から除去"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/163"
    - "../discovery/exec-contract-content-audit-removal.md"
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
  related:
    - "../specs/exec-contract-content-audit-removal.md"
    - "../designs/exec-contract-content-audit-removal.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/exec-contract-content-audit-removal.md"
  derived-by:
    - "../designs/exec-contract-content-audit-removal.md"
  verifies: []
  verified-by: []
  references:
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../discovery/permission-model-responsibility-redesign.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# exec-contract cleanup — 内容審査を mandatory 層から除去

## Context and Problem Statement

親 ADR（ADR-034KuqZelggTIb8Z8K9QB8・accepted）は「cheaper の
責務は宣言された実行契約の強制であり、宣言された executable の
内容を審査することではない」を確定した。本 ADR が決めるのは
その**除去をどう実装するか**（除去単位・deny predicate の
disposition・残置する viability 参照の扱い）であり、除去するか
否か・opt-in profile への降格という architecture 判断自体は
再決定しない。

mandatory 層に残る content-audit 7 項目（A8・A10・B3・B4・B5・
B6・F3）と A9 deny list の 3 適用点は、policy resolve・pin・
spawn 前再検証・task 検証・native 解決の複数層に分散し、互いに
同一の「内容審査」責務を重複して担っている。個別の検査を
部分的に残すと責務境界が再び曖昧になり、compensating check が
他層へ移るだけになる。

## Decision Drivers

* 親 ADR・親 design（approved）の責務分離モデルに整合すること。
* 除去後も pin 同一性（B2）・spawn 前再検証（B1）・shell-free
  argv spawn・lifecycle・filesystem/declared-resource 契約が
  維持されること。
* issue #164（`security.execDenyPatterns` opt-in profile）が
  除去した mandatory 適用点を前提に再配置できること。
* 除去は grep で検証可能な単位で行い、審査 helper の残骸を
  残さないこと。
* `route/resolve.rs:285` の viability 参照が `argv_shell_re` を
  必要とするため、regex 定義の削除は issue 4 まで延期すること。

## Considered Options

* Option A — 7 項目 + A9 適用点を一括除去する（本 ADR が採用）
* Option B — feature flag / 段階的 rollout で徐々に除去する
* Option C — 項目ごとに個別 issue・個別 PR で逐次除去する
* Option D（sub-decision: A9 predicate 定義の disposition）—
  call site のみ除去し `command_denied_executable_re`・
  `is_denied_command_executable` を残置する

## Decision Outcome

Chosen option: "**Option A — 一括除去**"、かつ A9 predicate 定義は
Option D の残置を採らず **call site と共に削除**する。理由:

* 7 項目は親 design で**同一の content-audit 責務クラス**として
  整理されており、層を跨いで相互に補完している。部分的に残すと
  「どの層が審査するか」の境界が再び曖昧になり、compensating
  check が残存層へ移るだけで構造的問題が解消しない。
* 段階的・flag 制御は「mandatory か optional か」の knob を
  再導入することになり、親 ADR が棄却した demotion 判断と
  矛盾する。deny の optional 化は A9 に限定されており、それは
  issue #164 が operator 宣言 pattern で**別 predicate**として
  実装する。
* builtin regex `command_denied_executable_re` は定数であり、
  issue #164 の opt-in profile は `security.execDenyPatterns`
  の operator 宣言 regex を compile する**異なる predicate**
  を要する。残置しても再利用価値が無く、YAGNI を適用して削除
  する。旧既定値は issue #164/#169 の docs migration 作業で
  「旧既定値」として記録する。

### Consequences

* Good, because 認可層が「宣言契約の強制」のみを担う単一責務に
  なり、7 項目 + A9 の除去が grep で検証可能な一意の scope に
  なる。
* Good, because `python`・`ssh`・`tar`・`.sh`・workspace-resident
  binary・bare `node` の宣言 grant が opt-in deny profile 無しで
  reject されなくなり、policy モデルが「宣言したものがそのまま
  実行契約」になる。
* Good, because pin 取得・`resolved != pinned` の spawn 前再検証
  ・`tokenize_shell`・wildcard 構造検査・lifecycle・
  `PathDenylist` 等の真の実行契約は Keep Site Inventory で
  明示的に温存される。
* Bad, because pin 済み executable の**内容差替**（同 path の
  非 native 化）は B3 除去で検出対象外になる。path/symlink
  retarget は pin-equality で検出維持されるが、内容差替の
  検出は cheaper の保証対象外になることを docs で明記する。
* Bad, because `\0`・metachar を含む argv が policy 層の
  `POLICY_VIOLATION` でなく spawn/OS 層のエラーに変わり、
  error 分類が縮退する。拒否自体は維持されるが観測差分として
  記録する。
* Bad, because shebang 無し +X text file が glibc `execvpe` の
  `/bin/sh` fallback で解釈され得る。spawn API 層の shell-free
  は維持されるが、fallback は OS/libc 既定動作であり cheaper の
  保証対象外と明記する（親 design B3 補足）。

### Confirmation

* spec の Acceptance Criteria（`SPEC-034KI3mWcW92kMljP5xoa8`）
  通りに実装され、各 grep 確認・test・`mise run check` が
  通ること。
* 除去 helper が残存しないことを grep で確認する。
* `route/resolve.rs:285` の `argv_shell_re` viability 参照が
  残置されていること。

## Implementation Plan

* Affected paths:
  * `crates/cheaper-core/src/policy/process.rs` —
    `shell_executable_re`・`script_ext_re`・
    `is_shell_process_executable`・task loop の 2 適用点除去。
    `argv_shell_re` 定義は残置。
  * `crates/cheaper-core/src/policy/command.rs` —
    `ArgPattern::compile` の metachar 審査・
    `resolve_command_policy` の shell/deny 適用・
    `command_denied_executable_re`・`is_denied_command_executable`・
    `is_relative_executable_path` 除去。
  * `crates/cheaper-tools/src/tools/command/mod.rs` —
    `within_workspace`・pin/verify の deny・workspace 再査・
    `authorize_args` の metachar 拒否除去。pin-equality・
    `native_executable` 再解決は維持。
  * `crates/cheaper-tools/src/tools/proc/executable.rs` —
    `native_magic_supported`・`native()` 内 magic/shim 判定・
    front-door shim 判定・bare `node` reject 除去。win32
    `.exe/.com`・X_OK・`native_executable` wrapper は維持。
  * `crates/cheaper-tools/src/tools/process/prepare.rs` —
    `resolve_process_executable` 経路の message restate。
  * 各 test/fixture（discovery の Affected Tests / Fixtures
    Inventory 通り）。
* Patterns to follow: pin-equality・spawn viability・wildcard
  構造検査の残存判定はそのまま残し、審査分岐のみを除去する
  「残る判定は残す」最小変更。`route/resolve.rs` は変更しない。
* Patterns to avoid: 削除した審査を別条件で再実装しない、
  `tokenize_shell` を実行拒否に転用しない、`argv_shell_re`
  定義を削除しない、viability 側の unsafe 分類を本 issue で
  変更しない。
* Dependencies or configuration: 依存追加・schema 変更・feature
  flag は無し。issue #164 が `security.execDenyPatterns` を
  別途導入する。

## Verification

* [ ] `is_shell_process_executable`・`native_magic_supported`・
  `within_workspace`・`is_relative_executable_path`・
  `script_ext_re`・`command_denied_executable_re`・
  `is_denied_command_executable` が `crates/` から grep で
  消失すること。
* [ ] `argv_shell_re` 定義と `route/resolve.rs:285` の参照が
  残置されていること。
* [ ] `python`・`ssh`・`tar`・`.sh`・workspace-resident・
  bare `node` 宣言が policy resolve・pin で拒否されない test
  が追加されていること。
* [ ] pin-equality（path 差替検出）の spawn 前再検証 test が
  B3 非依存で維持されていること。
* [ ] `mise run check` が通ること。

## Pros and Cons of the Options

### Option A — 一括除去

* Good, because content-audit 責務クラスが一括で mandatory 層から
  消え、残存審査が無いか grep で検証可能。
* Good, because 親 design の 7 項目を跨ぐ相互依存（A8/F3 の
  shim predicate・B3/F3 の native 検査・A9 の 3 適用点）が
  一括で解消される。
* Bad, because 1 commit の diff が複数 crate・多数の test
  churn を含み、レビュー単位が大きい。

### Option B — feature flag / 段階的 rollout

* Good, because 挙動変化を段階的に適用でき rollback が容易。
* Bad, because 「審査するかしないか」の knob を再導入し、親
  ADR が棄却した mandatory/optional の混在を再現する。
* Bad, because flag 両経路の test・docs・error message を維持
  するコストが高く、審査ロジック自体が残るため責務分離の
  構造的問題が解消しない。

### Option C — 項目ごとの個別除去

* Good, because 各 PR が小さくレビューしやすい。
* Bad, because 中間状態で一部項目が残り、責務境界が曖昧な
  まま複数 issue に跨る。A9 適用点と A8 shim predicate は
  `is_shell_process_executable`/`is_denied_command_executable`
  を共有しており、逐次除去では相互依存の分岐を一時的に残す
  必要がある。
* Bad, because 「宣言 executable が拒否されない」という
  acceptance が部分除去中は成立せず、後続 issue の前提が
  安定しない。

### Option D — A9 predicate 定義を残置

* Good, because issue #164 で opt-in deny profile を再実装する
  際に predicate 名を残せる。
* Bad, because `command_denied_executable_re` は builtin 定数
  であり operator 宣言 pattern を compile する #164 の
  predicate と構造が異なり、実質的に再利用できない。残置は
  YAGNI に反する dead pub code になる。→ call site と共に
  削除することを選択。

## More Information

* 起点 issue: https://github.com/scarecrow173/cheaper/issues/163
* 後続 issue: https://github.com/scarecrow173/cheaper/issues/164
  （opt-in deny profile の再配置先 — builtin 一覧の docs 記録含む）
* 派生元 discovery:
  `../discovery/exec-contract-content-audit-removal.md`
  （実測 file:line inventory・keep list・観測差分）
* 根拠文書（`references`/`related` のみ — cross-chain lineage は
  張らない）:
  * `../adr/enforce-declared-contracts-over-content-audit.md`
    （architecture 判断の確定元 — 本 ADR はその実装形のみ決める）
  * `../designs/permission-model-responsibility-redesign.md`
  * `../specs/permission-model-responsibility-redesign.md`
  * `../discovery/permission-model-responsibility-redesign.md`
* 兄弟 spec: `../specs/exec-contract-content-audit-removal.md`

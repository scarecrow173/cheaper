---
id: "TSPEC-034KJtUqMNXjkbjsbz5YPa"
type: "test-spec"
status: "draft"
title: "exec-contract 内容審査除去の検証契約 (issue #163)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/163"
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
    - "docs/plans/exec-contract-content-audit-removal.md"
    - "docs/discovery/exec-contract-content-audit-removal.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/exec-contract-content-audit-removal.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
    - "docs/adr/exec-contract-content-audit-removal.md"
  derived-by: []
  verifies:
    - "docs/specs/exec-contract-content-audit-removal.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
  verified-by: []
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/discovery/permission-model-responsibility-redesign.md"
    - "crates/cheaper-core/src/policy/process.rs"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/tools/proc/executable.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
  defers: []
  deferred-by: []
---

# exec-contract 内容審査除去の検証契約 (issue #163)

## Purpose

issue #163(#151 実装分解 1/7)で確定した「宣言された実行契約のみを
強制し、宣言された executable・argv の**内容を審査して reject**
しない」という責務分離を、grep で確認可能な消失 pin・受理 pin・
観測差分 pin の 3 面で固定する。SPEC-034KI3mWcW92kMljP5xoa8
R1-R12 と DESIGN-034KIsSCvVMAxGDZY1dQTd の Removal Contract・
Keep-Boundary Pins が authoritative。

保つべき意図は 3 つ。(1) **除去の完全性**: 8 つの審査 helper が
名前ごと消え、適用点の残骸・seam・stub を残さない — 「どの層が
審査するか」の曖昧さが再発しない。(2) **契約強制の温存**: pin
同一性・wildcard 構造・cwd/env 境界・`PathDenylist`・spawn
viability・lifecycle が変わらず残り、keep site を誤って削らない。
(3) **観測差分の正直な記録**: `\0` argv の error 分類縮退・内容
差替非検出のように、除去で変わる観測面を黙って変えず pin で
明文化する。

退役条件: issue #164 の `security.execDenyPatterns` opt-in profile
が入っても本 TSPEC の核心(内容審査が mandatory 層に存在しない・
宣言契約のみ強制)は存続する。viability 側の審査依存整理(issue
4)で `argv_shell_re` 定義自体が消えた場合、`argv_shell_re` 残置
Rule のみ見直し対象。

## Feature

mandatory 実行契約層(grant/task resolve → pin → spawn 前再検証 →
argv 認可 → `native()`/`resolve_process_executable`)からの
content-audit 7 項目(A8・A10・B3・B4・B5・B6・F3)+ A9 deny
list 3 適用点の一括除去と、それを観測する golden/unit/
integration/dev-seam/regression test の張替え。

## Rules

- Rule: **name-disappearance grep** — 次の 8 名が `crates/` から
  消失すること: `is_shell_process_executable`・
  `native_magic_supported`・`within_workspace`・
  `is_relative_executable_path`・`script_ext_re`・
  `shell_executable_re`・`command_denied_executable_re`・
  `is_denied_command_executable`。確認コマンド:

  ```bash
  grep -rn "is_shell_process_executable\|native_magic_supported\|\
within_workspace\|is_relative_executable_path\|script_ext_re\|\
shell_executable_re\|command_denied_executable_re\|\
is_denied_command_executable" crates/
  ```

  許容 residual: `within_workspace` は `walker.rs`/
  `normalization.rs` の無関係な test 名
  (`..._under_within_workspace`)にのみ hit してよい。除去対象は
  `command/mod.rs` の fn とその呼出しであり、test 名の残存は
  許容する(design Verification Notes)。

- Rule: **stale-vocabulary grep(spec R10)** — error message・doc
  comment・test comment に除去した審査の旧語彙を残さない:

  ```bash
  grep -rniE "deny|workspace-resident|shim|native magic" crates/
  ```

  残存 hit を全て確認し、許容するのは (a)
  `denyPaths`/`PathDenylist`/`denied_declared_path` 等の denylist
  機構(E2 — executable deny とは別物)、(b) hook/permission 判定の
  `deny` verdict・capability/empty-grants 由来の `denied` test 名、
  (c) 「shim」の非該当用法(`serde_yml` shim・PATH shim・
  self-reshim)、(d) `route/resolve.rs:279-281` の viability
  comment(前提は issue 4 完了まで一時的に偽 — known-stale として
  残置し comment churn 対象外)のみ。

- Rule: **declared-name acceptance** — `python`/`python3.13t`/
  `env`/`ld-linux-x86-64.so.2`/`bash`/`deploy.sh` を含む grant が
  `resolve_command_policy` を PATH 非依存で通り、`python`/`ssh`/
  `tar`・workspace-resident(relative `./tool`・absolute
  inside-workspace)・resolved basename が旧 deny 名の symlink が
  opt-in deny 未指定で policy resolve・pin で拒否されない
  (R1/R2)。受理は「何も審査しない」ではなく wildcard 構造・
  cwd/env・lifecycle 契約は引き続き強制する。
- Rule: **`\0` degradation** — `"a\0b"` を含む argv は policy 層
  `POLICY_VIOLATION` でなく spawn 層 `TOOL_EXECUTION_FAILURE` +
  `details.spawnCode="SPAWN_ERROR"` として観測される(R6)。
  `\0` **以外**の metachar(`\r`・`\n`・`|&;<>`・`` ` ``・`$(`)
  は literal argv として子へ届き spawn 自体は成功する。
  pin は code+spawnCode のみ assert し message 非依存とする。
- Rule: **declared executable 内 `\0` は縮退しない** — `\0` を
  含む executable は realpath 失敗で pin 層 `POLICY_VIOLATION`
  のまま(spawn 層へ到達しない)。`env` の NUL 規約と
  `relative()` の `'\0'` 拒否は別規約として維持する。
- Rule: **`a*b` repurpose determinism** — deny list 前提だった
  test の再用途先は `argv:["a*b"]`(wildcard 構造違反 →
  `ArgPattern::compile` の `INVALID_RUNTIME_POLICY`)へ張り替え、
  invocation 層 code・`details.layer="invocation"`・inner code 保持
  を PATH 非依存で deterministic に pin する(`load.rs`
  unit・`load.json` golden・`run_dev_seam` case・`proc_degrade`
  spec・regression case)。`policy_grant_add` 経路の別再用途は
  `{"executable":"git","timeout":"5x"}`(bounded duration 違反)で
  同 code を pin する。
- Rule: **`argv_shell_re` 残置** — regex 定義
  (`policy/process.rs`)と `route/resolve.rs:285` の viability
  UnsafeCommand 分類・`is_interactive`・`CWD_CHANGING_COMMANDS`
  は変更しない(R8)。定義の doc comment は「`route/resolve.rs`
  viability が参照する間 `pub` を維持」へ訂正されること。
- Rule: **keep-boundary 不変** — `tokenize_shell`(viability 分類・
  拒否ではない)・`resolve_process_executable` の PATH 検索→
  canonical realpath・`native_executable` 再解決・
  `resolved != pinned` pin-equality・wildcard 構造検査+
  `ArgPattern::matches`・`relative()`・env dup/NUL・lifecycle
  bound・nonempty grants/tasks gate・`PathDenylist`/
  `denied_declared_path`・win32 `.exe/.com`・X_OK・
  `spawn_failure_is_async`・`env_clear`/stdin null/process group
  の code site は変更しない(R9)。
- Rule: **pin-equality の検出範囲再設計** — spawn 前再検証は
  path 差替(symlink retarget・realpath 不一致・pinned file 消失)
  を B3 非依存で検出し、同 path の**内容差替**は検出対象外である
  ことを positive assert(差替 file が検査を通り実行まで進む)。
  symlink retarget は unix のみ、file 消失→`"no longer resolves"`
  は全 platform で pin。
- Rule: **bare `node`・script の受理境界** — bare `node` は PATH
  上にあれば通常解決され(`search_path` 注入・mise toolchain の
  ambient PATH の両経路で pin)、無ければ解決不能として通常の
  unsupported 経路(R5 — 受理しないのではなく解決不能)。
  script/shim file は unix では +X で解決されるが、win32 では
  `.cmd`/`script` 等の非 `.exe/.com` 拡張子は引き続き spawn
  viability で reject される(platform 分岐 pin)。
- Rule: **再実装・補償 check の非存在(R11)** — 削除した審査を
  別条件・別層で再実装する分岐、`tokenize_shell` の実行拒否転用、
  #164 のための seam/flag/stub が diff に含まれない。
- Rule: **error contract の残存面** — `POLICY_VIOLATION` の
  unknown grant/task・argv pattern mismatch・`"no pinned
  executable"`・`"pinned executable path changed since pin"`・
  denylist 由来 `denied_declared_path`・WorkspaceEscapeError、
  `INVALID_RUNTIME_POLICY` の schema 違反・duration 上限・
  wildcard 構造違反・`relative()` 規約・env dup/NUL は不変。
  `unsupported()`/`unsupported_empty_command`/pin 再解決失敗の
  message は `"{subject} could not be resolved to a spawnable
  executable"` 系へ restate(`native`/`shell`/`shim` 語彙を残さない)。

## Examples

- Example: `resolve_command_policy({"executable":"python"})` が
  PATH 非依存で受理される(現行: declared basename deny で
  INVALID_RUNTIME_POLICY)。`{"executable":"python","argv":["a*b"]}`
  は wildcard 構造違反で引き続き `INVALID_RUNTIME_POLICY` —
  invocation 層なら `details.layer="invocation"` + `"invalid
  command policy"` inner code を保持する deterministic pin。
- Example: tempdir 内 `bin/tool` を実在させた declared `./tool` と
  workspace 内 absolute path の grant が pin・runtime 構築を通る
  (現行: `is_relative_executable_path` pre-filter と
  `within_workspace` 再査で POLICY_VIOLATION)。
- Example: symlink `tool`→実 file `python3`(cfg unix)の pin が
  受理される(現行: resolved basename deny で POLICY_VIOLATION)。
- Example: `authorize_args` に metachar token(`"a;b"`・`"a|b"`・
  `"v$(i)"`)が rest marker pattern で受理され、pattern 不一致だけ
  が `POLICY_VIOLATION` になる。`"a\0b"` は子 spawn で
  `TOOL_EXECUTION_FAILURE`+`SPAWN_ERROR` へ縮退する。
- Example: `["bash","-c","x"]`・`["run.sh"]`・`["echo","a|b"]` の
  process task が `resolve_process_policy` を通る(golden
  `process.json` の受理 expected)。`command:["sh"]` task が
  `validatePolicySchema`(`mode:complete`)を通り validated echo を
  返す(golden `schema.json`)。
- Example: tempdir `bin_dir` に `node`/`node.exe`(+X)を配置した
  `search_path` で bare `node` task が解決され、`Some("")`(PATH
  遮断)では unsupported error — B5 除去の end-to-end pin は
  `["node","-e",<script>]` task が mise toolchain の実 `node` で
  実行されること。
- Example: cfg(unix)で pinned file を symlink へ置換 → `"path
  changed since pin"`(realpath 不一致)。全 platform で pinned
  file 削除 → `"no longer resolves"`。同 path への非 native 内容
  差替は検査を通り実行まで進む(検出対象外の positive pin)。
- Example: `verify_pinned_executable(name, pinned)` が `root`
  引数を持たない签名になり、missing path は error(restate 後
  message)・workspace 内 file は `resolved == pinned` で検査通過。
- Example: regression `run-grant-deny` → `run-grant-invalid-spec`
  rename 後、`--grant {"name":"deny-me","executable":"python",
  "argv":["a*b"]}` が exit 3 + `INVALID_RUNTIME_POLICY` +
  `details.layer="invocation"` を返す(expected の内容は不変 —
  code+layer のみ pin で message 非依存)。

## Guarantees

- 実装者が design を再解釈しなくても、name-disappearance grep の
  8 名・stale-vocab grep の残存 hit 集合・受理 pin の executable
  名集合・`\0` degradation の code+spawnCode・`a*b` repurpose の
  deterministic 性が unit+integration+golden+dev-seam+regression
  test と grep で機械的に検証できる。
- `python`/`ssh`/`tar`/`.sh`/workspace-resident binary・bare
  `node` を含む宣言が opt-in deny 未指定で policy resolve・pin・
  task resolve・runtime 構築を通ることが、各層の test で pin される。
- keep-boundary(pin 同一性・wildcard 構造・`relative()`・env
  NUL/dup・`PathDenylist`・spawn viability・lifecycle)の既存
  contract test が変更なく通り、誤削除は grep-pin で検出される。
- error message・doc comment が除去後の実態を正直に記述し、
  `native`/`shell`/`script shim`/`denied`/`workspace-resident`/
  `metachar` の旧語彙が残らない(allowlist 外 hit ゼロ)。
- `#164` が再挿入する 3 適用点(declared basename・resolved
  basename・spawn 前再検証)の構造が、seam/stub なしの自然な
  if-block 痕跡として残る。

## Non-goals

- `security.execDenyPatterns` schema・opt-in deny predicate・builtin
  旧既定値の docs 記録(issue #164/#169 の scope — 本検証では
  opt-in deny の非存在のみを確認)。
- `route/resolve.rs:285` の viability unsafe 分類・`tokenize_shell`
  の評価・viability 評価順の見直し(issue 4 scope — 残置の
  `argv_shell_re` 定義と参照が不変であることのみを検証)。
- ENOEXEC → libc `/bin/sh` fallback の保証 — shebang 無し +X text
  file が execvpe 系で shell 解釈され得るのは OS/libc 既定動作で
  cheaper の保証対象外(issue 7 で docs 化予定 — 本検証は spawn
  API 層の shell-free のみを前提にし、fallback 発生有無は問わない)。
- 同 path の pin 済み executable 内容差替の検出 — B3 除去で検出
  対象外になる design 受理済み trade-off(非検出であること自体を
  positive assert するが、検出能力の検証ではない)。
- grant/task merge・substitute mode・trusted flag・docs 全体
  sweep(後続分解 issue の scope)。
- `route viability` の挙動変化レビュー — workspace-resident・
  shim・旧 deny 該当の declared grant が pin を通り NoGrant →
  Runnable になる経路の増加は観測するが分類の見直しは issue 4。
- 検証コマンド・実行結果の記録 — task の `## Verification` と
  impl record の責務。

## Risk

- name-disappearance grep を pin しないと、helper 残骸や別名での
  審査再実装が「除去済み」のまま残り、mandatory/opt-in の責務
  分離が形骸化する — grep で検証可能が ADR の除去単位定義。
- `\0` degradation pin を張らないと、error 分類の縮退
  (POLICY_VIOLATION → TOOL_EXECUTION_FAILURE)が consumer へ
  undocumented breaking change として漏れる。
- `a*b` repurpose を PATH 非依存で書かないと、deny 前提だった
  test が環境依存 flake 化する(`python` が PATH にある CI と
  無い環境で結果が分かれる)。
- 受理 pin(declared name・workspace-resident・symlink basename)
  を張らないと、残存層への補償 check 混入(例: pin 層に basename
  判定が残る)を検出できず、ADR が棄却した「部分的残置」へ
  regress する。
- keep-boundary の grep/test を通さないと、wildcard 構造・env
  NUL・pin-equality・denylist といった宣言契約を巻き添え削除して
  も気づけない(除去過多は「審査が残る」より静かに壊れる)。
- `spawn_time_revalidation` を内容差替のまま残すと B3 除去で
  false-green(差替を検出しないのに検出名のまま)になる —
  path 差替へ再設計しないと pin-equality の保証範囲を誤表示する。
- `node` positive pin を toolchain 前提なしで書くと mise 非経由の
  実行環境で flake になる(Risk#2 と同根 — 前提明記が mitigation)。

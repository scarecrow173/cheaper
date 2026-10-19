---
id: "DESIGN-034ILuADFr70oZoUROR13q"
type: "design"
status: "approved"
title: "hook-config legacy conflict gate demotion to warning"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/128"
    - "https://github.com/scarecrow173/cheaper/issues/127"
    - "https://github.com/scarecrow173/cheaper/pull/131"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
      - type: "file"
        path: "scripts/test/devin-hooks-install.test.mjs"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "docs/guides/distribution.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "./hook-eval-dedup-verdict-cache-and-leader-election.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "../../crates/cheaper-cli/src/hook_config.rs"
    - "../../crates/cheaper-cli/src/hook/eval_cache.rs"
    - "../../crates/cheaper-cli/tests/integration/hook_config.rs"
    - "../../scripts/test/devin-hooks-install.test.mjs"
    - "../guides/distribution.md"
    - "../../packages/apm/.apm/skills/cheaper-usage/SKILL.md"
  defers: []
  deferred-by: []
---

# hook-config legacy conflict gate demotion to warning

## Context

issue #127 の hook eval dedup（canonical request fingerprint による
verdict cache + leader election、PR #131、
`crates/cheaper-cli/src/hook/eval_cache.rs`）により、複数登録 path
（`.devin/config.json` native + `.claude/settings.json` import 等）
から発火する cheaper hook は、fingerprint が一致する同一 canonical
request の間で評価 1 回・全 firing 同一 verdict へ収束する。
これにより現行 legacy conflict gate の前提（「重複発火は unsafe
なので未然に防ぐ」）は除去され、既存 cheaper 登録を持つ consumer
project で `hook-config sync` が block される問題（issue #128）
を解消するために、gate を blocker から warning へ降格する。

本 design は SPEC-034IKGkW4dQEVw6QigMFxQ（requirements R1–R11・
exit code 表・Acceptance Criteria — 拘束力のある contract）と
ADR-034IKJq6tKWWAep0Q0ONzK（Option 1「gate を warning へ降格」の
構造決定）を、`crates/cheaper-cli/src/hook_config.rs` の touch
point 単位で実装可能な精度へ落とすものである。

降格メカニズムの要点（spec contract どおり）:

- **検出は維持・深刻度のみ降格**: `legacy_conflicts()`
  （`hook_config.rs:362-394`）・`KNOWN_LEGACY_FILES`（:42-49）・
  `CONFLICT_PATTERN`（:38）の検出規則は一切変更しない。変わるのは
  各 command が検出結果をどう表面化するかのみ。
- `sync`: `invalid_request` early return を除去し upsert を継続。
  result に `legacyConflicts`（常時）+ `warning`（conflict 有り時
  のみ）を載せて exit 0（R1-R3）。
- `verify`: conflict を `blocker` から外し informational
  `legacy-conflict:` finding とする。`clean` は blocker 系
  finding のみから導出（R4-R6）。
- `remove`: `residualRegistrations` + informational note の構造は
  維持し、note 文言のみ整合（R7）。
- `--event-stdin`: 構造変更なし。verify exit code passthrough により
  `post-install`/`post-update` が conflict 有りでも exit 0 で収束し、
  `post-uninstall` の `packages` 判定不能 branch は conflict-only
  residual で exit 2→0 へ変わる（R8）。
- flag 追加・severity field・auto-cleanup・所有境界の変更はしない。

## Scope

### In Scope

- `crates/cheaper-cli/src/hook_config.rs` —
  `run_verify`（conflict 処理 :668-682・`remediation` :730-738）、
  `run_sync`（doc-comment :742・gate :757-768・result 組立
  :850-860）、`run_remove`（residual note :919-939）、
  `legacy_conflicts` の stale comment（:376-378）の変更。
- `crates/cheaper-cli/tests/integration/hook_config.rs` —
  :132 の書き換え + event-stdin×conflict・`--quiet`・retained error・
  remove residual の test 追加。
- `scripts/test/devin-hooks-install.test.mjs` — :269-285 の旧 gate
  assert の書き換え。
- `docs/guides/distribution.md` — sync 安全条件（:130-141）・
  lifecycle 節（:148-169）・移行節（:171-182）の整合 + 旧 binary
  caveat 追記（R11）。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — :167-171 の
  旧 gate 記述と :194-199 の post-uninstall 段落の整合。
  更新対象は配布 source の同 path のみ — deploy 写しは repo 内に
  存在しない（consumer 側生成物）。

### Out of Scope

spec の Out of Scope をそのまま引き継ぐ:

- conflict 検出時の foreign registration 自動除去（auto-cleanup）—
  own-entry-only 所有境界を越える。
- `--allow-conflict` 等の flag — `--event-stdin` に flag 伝達路が
  無く dead weight（issue #128 で却下済み）。
- capability gate（PATH 上の cheaper が `hook targets --json` で
  devin target + `hookConfig` を報告する check）の変更。
- dedup 機構・verdict 意味論・canonical matcher / own entry shape
  の変更。
- `.devin/hooks.v1.json` の生成・rewrite（`shadow` finding は
  informational のまま維持）。
- 旧 binary（dedup 無し）への実行時対策 — docs 記述のみの
  mitigation（R11）。
- 歴史的 doc（旧 blocker semantics を記述する
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md:144`・
  `docs/tasks/cli-owned-hook-config-lifecycle.md:81-82`）の遡及更新。
- `install-devin-hooks.ps1` wrapper 自体の変更 — thin wrapper は
  sync の envelope を成功時に echo しないが、spec の scope 外。
  なお `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1:5`
  の header comment は「legacy conflict gate」という表現を残して
  おり降格後は stale となる — wrapper 自体が scope 外のため変更
  せず、drift として記録する。

## Components and Boundaries

変更は `hook_config.rs` 内の 4 action + 2 comment（:377・:742 の
「legacy gate」系 stale 記述）に閉じる。
所有境界（own entry = `OWN_COMMAND` 完全一致 group のみ管理、
他 agent / user entry 不触）は不変。`legacy_conflicts()` は
informational source として据え置き — 各 command がその結果を
result map / findings へ載せる形だけを変える。

## Detailed Design

### `run_verify` — conflict を blocker path から外す（:668-682）

現行:

```rust
let conflicts = legacy_conflicts(&root_dir, &config_path);
if !conflicts.is_empty() {
    blocker = true;                       // ← 除去
    for f in &conflicts {
        findings.push(...旧文言...);
    }
}
result.insert("legacyConflicts", ...);    // 常時 populate(変更無し)
```

新: `blocker = true` を削除し、per-file finding push と
`legacyConflicts` populate は維持する。

```rust
// legacy conflict scan — informational(dedup により共存は安全)。
let conflicts = legacy_conflicts(&root_dir, &config_path);
for f in &conflicts {
    findings.push(Value::from(format!(
        "legacy-conflict: {f} contains a cheaper hook registration \
         (safe to keep — duplicate firings are deduplicated at eval \
         time for identical requests; remove that registration to \
         converge on a single registration)"
    )));
}
result.insert("legacyConflicts", ...);    // :679-682 そのまま
```

- `clean`（:729 `!blocker`）は blocker 系 finding
  （`config-unreadable` :615・`missing` :661-664・`modified` :646・
  `duplicate` :653-655・`absent-config` :721-725）のみから導出される
  構造のまま — 新たな severity field は導入せず、informational は
  prefix 規約（`matcher-drift`/`shadow`/`orphan`/`legacy-conflict`）
  で識別する（R6）。informational finding と `clean:true` の併存は
  `matcher-drift` が既に示す既存 semantics。
- `remediation`（:730-738）は `!quiet && blocker` gate を維持し、
  文言のみ書き換え（Wording Contract 参照）。conflict-only 状態では
  blocker が立たないため `remediation` 自体が emit されない。

### `run_sync` — early return 除去 + result field 追加（:757-768・:850-860）

現行 gate（:760-768 の `return Err(invalid_request(...))`）を除去し、
scan 結果を保持して result へ載せる。

**scan の位置 — write 前を採用**:

```rust
// legacy conflict は blocker ではなく informational — dedup が
// 重複発火を無害化するため upsert を継続し、結果で報告する。
let conflicts = legacy_conflicts(&root_dir, &config_path);
```

- 順序は observably identical: scan 対象は `KNOWN_LEGACY_FILES`
  （sync が一切書かない）+ target file で、target file 内の own
  command は :384-388 の除外規則で既に conflict 対象外 — sync が
  書いた own entry が自己検出されることはなく、scan が write の
  前後どちらでも結果は同じ。
- write 前を採る理由: (a) 「決定時点の snapshot」を報告するという
  verify の read-only semantics と対称的、(b) gate の位置に scan を
  残す方が最小 coherent diff で、(c) 書込み失敗（外部変更検出等）
  時に result を emit しない現行構造を自然に維持できる。
  scan-after-write に behavioral gain は無い。

result 組立（:850 付近、`changed`/`matcher` insert の直後・quiet
gated `note` の**外**）:

```rust
result.insert(
    "legacyConflicts".to_string(),
    Value::Array(conflicts.iter().map(|f| Value::from(f.clone())).collect()),
);
if !conflicts.is_empty() {
    result.insert("warning".to_string(), Value::from("…Wording Contract の文言…"));
}
if !quiet {
    result.insert("note".to_string(), ...); // 既存どおり quiet gate
}
```

- R1: `legacyConflicts` は result map を返す場合に**常に**含める
  （空 array も可）。key 名・shape（file path の string array）は
  verify result の同名 field と同一で表現ドリフトを起こさない。
  `ok:false` error envelope には result map が無く適用外。
- R2: `warning` は conflict 検出時のみ。`--quiet` に関わらず両
  field を insert する — `note` の quiet gate（:852-858）を複製
  しないことが guard 対象（test AC）。
- R3: 既存 error 条件は全て不変 — `check_capability`（:748）、
  `resolve_config_path`/`check_no_reparse`（:749-750）、
  `load_config` の `?`（:771）、`own_group_matcher` modified
  （:831）、duplicate own（:840-845）、`write_config_atomic` の
  外部変更検出（:848）。
- 副次的変化: `sweep_orphans`（:770）は early return 除去により
  conflict 有りの sync でも実行されるようになる（降格前は gate
  の先で unreachable）。orphan temp/bak の掃除が増えるだけの
  harmless/beneficial な side effect で、新たな contract 追加は
  意図しない。

### `run_remove` — note 文言のみ（:919-939）

`residualRegistrations` key・emit 条件・informational 構造は不変
（R7）。`note` の文言のみ新規約へ整合させる（Wording Contract）。

### `run_event_stdin` — 構造変更なし（:1001-1052）

- `post-install`/`post-update`（:1001-1007）は `run_sync` →
  `run_verify` を順に呼び verify code を透過する構造のまま。
  `result.sync` が R1/R2 の `legacyConflicts`・`warning` を得て、
  `result.verify` が informational finding を載せ、conflict-only
  では verify code 0 で収束する（R8）。sync 内部 error の `?`
  伝播（:1002）は non-finding error として exit 2 のまま。
- `post-uninstall` の `packages` 判定不能 branch（:1023-1036）は
  `run_verify` code を透過するため、conflict-only residual で
  exit 2→0 へ**変わる**。`packages` に cheaper が含まれる branch
  （:1010-1014 → `run_remove` が residual を informational 報告で
  exit 0）の現行規約と整合する意図的な behavioral change — code
  変更はなく、test で固定する。
- `Some(false)` noop・`pre-*`・unknown event noop・malformed
  payload error は現行維持。

### stale comment 修正 — `legacy_conflicts`（:376-378）+ `run_sync` doc-comment（:742）

:377 の comment「parse 不能 file は verify の finding で別途報告」
は stale — verify の `config-unreadable` は target config path
のみを対象とし（:611-617）、sibling file の parse 失敗は finding
にも `legacyConflicts` にも現れない。best-effort skip 規約自体は
変更せず、comment を実動作へ修正する:

```rust
let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
    // parse 不能な sibling file は best-effort 規約で黙って skip —
    // verify は sibling の parse 失敗を finding として報告しない。
    continue;
};
```

同様に `run_sync` の doc-comment :742（「canonical own entry の
冪等 upsert + legacy gate」）は gate 除去で stale となる —
「canonical own entry の冪等 upsert + legacy conflict の
informational 報告」等、降格後の責務を表す文言へ修正する。
:757-759 の「legacy gate:」comment は gate 本体と一緒に置き
換わるため別途 touch しない。

## Wording Contract

新文言は全て JSON-safe（1 行・引用符・改行を含まない）。`—`
（em dash）は既存 finding 文言（:675 等）と同じく使用可。

### verify `legacy-conflict:` finding（prefix 維持 — R5）

```text
legacy-conflict: {f} contains a cheaper hook registration (safe to keep — duplicate firings are deduplicated at eval time for identical requests; remove that registration to converge on a single registration)
```

- `{f}` は conflict file path（現行どおり per-file finding）。
- "deduplicated at eval time for identical requests" で dedup 由来の
  安全性を明示する — verdict 同一性の保証は fingerprint が一致する
  canonical request 単位であり、無限定の "deduplicated" では全
  firing に効くかの誤読を招く（normative string は intent-level
  の表現より精度を優先する）。
- "remove that registration" — `{f}` が target file 自身の場合
  "remove it" は file ごとの削除とも読めるため、除去対象が登録
  entry であることを明示する。
- 「単一登録への収束は optional」を併記 — 「除去が前提」と読ませ
  ない。`duplicate firing is unsafe` を含まない。

### sync `warning`（result field — R2）

```text
legacy cheaper hook registrations remain (see legacyConflicts); coexisting is safe — duplicate firings are deduplicated at eval time for identical requests, so removal is needed only to converge on a single registration. A pre-dedup cheaper binary on PATH still evaluates each firing independently (duplicate evaluation only — the verdict is identical); see docs/guides/distribution.md
```

- file path は `legacyConflicts` array が既に持つため `warning`
  には埋め込まない（DRY）。
- dedup による安全性 + 収束は optional + PATH 上の旧 binary
  caveat（duplicate evaluation のみ・verdict 同一）+ docs pointer
  を含む。stderr channel は設けず result JSON のみ（R2）。

### verify `remediation`（:730-738 — R5）

```text
run `cheaper hook-config sync` to converge the registration; see docs/guides/distribution.md for the APM lifecycle wiring
```

- 「conflict 解消が sync の前提条件」と読ませる現行文言を除去。
  blocker 別の個別案内は各 finding 文字列と sync 自体の error
  message が担うため、`remediation` は generic な収束案内に留める。

### `run_remove` `note`（:932-938 — R7）

```text
remove deleted only the canonical entry; other files still contain cheaper registrations (safe to keep — duplicate firings are deduplicated at eval time for identical requests); run `cheaper hook-config verify` to inspect
```

- key `residualRegistrations`・構造は不変。residual が「残っても
  安全」である旨を新前提に整合させつつ、観察先として verify を
  引き続き指す。

## Data and Control Flow

spec の exit code 表のうち本 design が変える行（他は現行維持）:

| command | state | exit | envelope |
| --- | --- | --- | --- |
| `sync` | conflict のみ | 0 | upsert 実行、`legacyConflicts` 非空 + `warning`（**変更** — 現行は `invalid_request` exit 2） |
| `verify` | conflict のみ | 0 | `clean:true`、`legacyConflicts` 非空、informational finding（**変更** — 現行は `clean:false` exit 2） |
| `--event-stdin` `post-install`/`post-update` | conflict のみ | 0 | `result.verify.clean:true`（**変更**） |
| `--event-stdin` `post-uninstall`（`packages` 判定不能） | conflict のみ residual | 0 | `run_verify` code を透過 — exit 2→0（**変更**） |

非変更系: `sync` の config 不在/malformed/modified/duplicate/
capability 失敗、`verify` の missing/absent-config/malformed/
modified/duplicate、`remove` 全行、`pre-*`/unknown noop、
malformed payload、および finding を経ない non-finding error
（`--event-stdin` 内の sync error `?` 伝播 :1002、arg/path 解決
error :592-593 等）は全て現行どおり `ok:false` exit 2 系統。

降格の副次変化: config 不在 + conflict 有りの状態では
`absent-config` と `legacy-conflict` が併発する。`absent-config`
（:720-726）は `loaded.is_none() && !blocker` で発火し、降格前は
conflict の `blocker = true` で抑止されていた — 降格後は
informational `legacy-conflict` と blocker `absent-config`
（exit 2・`clean:false`）が同時に findings へ出る。

## Test Design

### `crates/cheaper-cli/tests/integration/hook_config.rs`

既存 test の書き換え・追加（`fixture_with_cheaper_on_path` PATH
shim パターンを再利用）:

1. **:132 rewrite — `hook_config_sync_warns_on_legacy_conflict`**
   （旧 `hook_config_sync_fails_on_legacy_conflict`）:
   `.windsurf/hooks.json` に legacy 登録を残す setup は流用。
   - `sync` → `code == 0`、`env["ok"] == true`、
     `result.legacyConflicts` が len 1 で windsurf path を含む、
     `result.warning` が非空 string、config file が存在し own
     entry（canonical matcher + `OWN_COMMAND`）が書き込まれている。
   - `verify` → `code == 0`、`result.clean == true`、
     `legacyConflicts` populate、findings に `legacy-conflict:`
     prefix を持つ entry が有り、かつ全 finding が
     "duplicate firing is unsafe" を含まない。
2. **新規 `hook_config_event_stdin_converges_on_legacy_conflict`**:
   `.windsurf/hooks.json` conflict setup + stdin payload
   `{"event":"post-install","packages":["cheaper"]}` → `code == 0`、
   config 書込み、`result.sync.legacyConflicts` 非空・
   `result.sync.warning` 存在・`result.verify.clean == true`。
   続けて `{"event":"post-update","packages":["cheaper"]}` →
   `code == 0` に加え `config_path(&fx).exists()` と own entry
   （canonical matcher + `OWN_COMMAND`）の存在を assert する
   （post-update でも config が書き込まれ/維持されることを固定 —
   post-install と同一 mapping の収束をカバーする冪等経路でも
   conflict で落ちない）。
3. **新規 post-uninstall 判定不能 × conflict-only residual**:
   own entry 書込み済み + conflict 残りの状態で stdin
   `{"event":"post-uninstall"}`（`packages` 欠落）→ `code == 0`、
   `result.action == "skipped"`、`result.verify.clean == true`
   （:1023-1036 の verify passthrough 継承 — 降格前は exit 2）。
4. **新規 `hook_config_sync_quiet_keeps_conflict_fields`**: conflict
   setup + `sync --quiet` → `code == 0`、`result.legacyConflicts`
   非空・`result.warning` 存在、かつ `result.note` は不在
   （`warning`/`legacyConflicts` が quiet gate 対象外 — R2、note
   の gate を複製していないことの guard）。
5. **retained error assertions（conflict 併存下）**: `.windsurf`
   conflict を置いた fixture で、malformed `.devin/config.json` /
   未知 matcher の modified own entry / duplicate own entry の各
   条件で `sync` が `code == 2`・`env["ok"] == false` を返す —
   conflict 降格が既存 error 条件を緩めないことを固定（1 test に
   fixture 再 setup でまとめても良い）。
6. **verify blocker retention（全 blocker 系 finding）**:
   informational `legacy-conflict` が blocker を覆わないことと、
   降格後も全 blocker で `verify` が `code == 2`・`clean == false`
   を返すことを固定する（spec AC — blocker 全種の coverage）:
   - conflict + own entry missing → `code == 2`・`clean == false`、
     findings が `missing` と `legacy-conflict` の両方を含む。
   - own entry に余分な key（または未知 matcher）を持つ config →
     `code == 2`・`clean == false`、findings が `modified:` prefix
     を含む（:643-647 — legacy-conflict fixture と併存させても
     良い）。
   - own entry group を 2 件持つ config → `code == 2`・
     `clean == false`、findings が `duplicate:` を含む
     （:650-656）。
   absent-config・malformed（`config-unreadable`）等他 blocker は
   既存 test（:62-76・:222-254）が現行どおり担保。
7. **remove residual**: conflict setup + own entry 書込み済み →
   `remove` → `code == 0`、`result.residualRegistrations` 非空、
   `result.note` が新文言（"deduplicated at eval time" を含む）
   — `residualRegistrations` key 維持 + note 文言整合を固定。

### `scripts/test/devin-hooks-install.test.mjs`（:269-285）

"stops on legacy cheaper registrations in known sources" →
"warns on legacy cheaper registrations in known sources" へ書き換え:

- wrapper `-Apply` run → `r.code === 0`、
  `existsSync(configAt(root)) === true`、parse した config が
  canonical entry（`CANON_MATCHER` + `CANON_COMMAND`）を持つ。
- **warning の可観測経路**: wrapper は成功時に sync の envelope を
  echo しない（`install-devin-hooks.ps1:84-93` — `Write-Host $out`
  は `ConvertFrom-Json` 失敗時のみ）ため、`r.combined` への
  `warning` assert は不可能。warning surfacing は real CLI を直接
  spawn して assert する:
  `spawnSync(cliExe, ["hook-config", "verify", "--project-root", root])`
  → stdout JSON の `result.legacyConflicts` が windsurf file を含み、
  `result.findings` に `legacy-conflict` entry、`result.clean === true`。
  verify は `check_capability`（PATH 上の cheaper）を持たないため
  PATH 加工なしで動き、この verify-spawn assert だけで spec AC は
  充足する。
- `warning` field 自体を見たい場合は `sync` を direct spawn する
  （idempotent なため再実行しても `warning` は返る）。ただし
  `sync` は `check_capability` → `find_on_path` で PATH 上の
  cheaper binary を要求するため、CLI dir を PATH に含めて渡す
  必要がある:

  ```js
  spawnSync(cliExe, ["hook-config", "sync", "--project-root", root], {
    env: {
      ...process.env,
      PATH: `${path.dirname(cliExe)}${path.delimiter}${process.env.PATH}`,
    },
  });
  ```
- 旧 assert（非ゼロ exit・config 非作成・`/legacy|conflict/i`）は
  削除 — 新 contract では全て偽になる。
- win32 gating 注意: describe block は
  `{ skip: !runPsAvailable || !cliBuilt }` で Windows + built
  `target/debug/cheaper(.exe)` 必須。本 repo は Windows-hosted かつ
  `mise run check` が build 後に `node --test` を流すため実効するが、
  非 Windows 環境では skip される（既存 coverage model どおり）。

## Documentation Design

### `docs/guides/distribution.md`

- **:135-138（`sync` の安全条件 bullet）**: 「conflict file を報告
  して停止する — 二重発火を防ぐため利用者が明示的に解消する」→
  conflict 検出は `legacyConflicts` + `warning` として result に
  報告し upsert は継続される旨へ改訂。重複発火は eval-time dedup
  で無害化されるため共存は安全で、単一登録への収束は optional と
  明記。
- **:148-169（APM lifecycle 節）**: `post-install`/`post-update` が
  conflict 有りでも verify exit 0 passthrough で収束する旨を追加。
  さらに旧 binary caveat を追記（R11）— PATH 上の cheaper binary
  が dedup 導入前の version の場合、重複登録は firing 毎に独立
  評価されるが影響は duplicate evaluation（token 計測・bounded
  materialization・bounded scan の重複）のみで verdict は同一・
  correctness は維持される（`eval_cache.rs` module doc :4-7）。
- **:171-182（移行節）**: 「旧登録が再 import され conflict gate
  が発火しうる → import 無効化か entry 除去」→ 再 import された
  旧登録は warning として報告されるのみで sync/lifecycle を
  block しない旨へ改訂。単一登録へ収束したい場合の除去は
  optional 手順として残す。

### `packages/apm/.apm/skills/cheaper-usage/SKILL.md`

- **:167-171**: "`sync` refuses to write while a legacy cheaper
  read-hook registration coexists … resolve duplicates explicitly"
  → sync は legacy 登録と共存しても canonical entry を書き込み、
  `legacyConflicts` + `warning` で報告する旨へ書き換え。dedup が
  重複発火を eval 時に無害化するため共存は安全で、単一登録への
  収束は optional（`/hooks` で全 loaded source の確認案内は維持）。
- **:194-199（post-uninstall 段落）**: residual な legacy 登録は
  informational で lifecycle を失敗させない（`packages` 判定不能時
  も verify passthrough で exit 0）旨を追記し、`verify` が残余状態
  を報告する fallback である記述を維持。

## Error and Edge Handling

- **non-finding error は不変**: capability gate 失敗・malformed /
  unreadable config・modified / duplicate own entry・外部変更検出・
  symlink/reparse 経路・arg/path 解決 error は従来どおり
  `?` 伝播 → `ok:false` envelope → exit 2。conflict 降格はこれら
  を緩めない（R3 — test #5 で固定）。
- **best-effort scan**: `legacy_conflicts()` は unreadable /
  unparseable な sibling file を黙って skip（:372-378）。malformed
  な legacy file 内の登録は `legacyConflicts`・finding のいずれに
  も現れない best-effort 規約は変更せず、:377 の stale comment のみ
  実動作へ修正。
- **write 失敗時の reporting**: scan-before-write のため write が
  error 伝播した run では conflict 情報は emit されない — error
  envelope には result map が無く、R1 の「常に」は success result
  に限るため contract 違反ではない。
- **idempotent sync**: `changed:false` の run でも `legacyConflicts`
  （+ 条件付き `warning`）は emit される — R1 の「常に」を冪等
  経路にも適用。
- **envelope**: `utility_success_envelope` 経由の通常 contract は
  不変。`warning`/`legacyConflicts` は result object 内 field で
  stderr channel は設けない（R2）。

## Non-Functional Constraints

- **所有境界不変**: `is_own_group`/`OWN_COMMAND` identity・
  strict-shape remove・atomic write・外部変更検出は全て据え置き。
- **新規面を増やさない**: flag・env var・依存・severity field・
  stderr 出力は追加しない（YAGNI — findings は prefix 規約の
  string array のまま）。
- **表現ドリフト回避**: sync result は verify result の既存 field
  名 `legacyConflicts`（string array of file path）をそのまま使い、
  同一概念に別名を作らない。
- **conflict 情報の可観測性維持**: exit code から外れる分、
  result field・finding・note の文言で stale 登録の存在を隠さない。

## Risks and Trade-offs

- **旧 binary caveat（受容）**: PATH 上の cheaper が dedup 導入前
  version の場合、重複登録は firing 毎に独立評価（duplicate
  evaluation — token 計測・bounded materialization・bounded scan の
  重複のみ、verdict は同一・correctness 維持）。runtime 対策は
  scope 外で、`warning` 文言内の docs pointer + distribution.md の
  caveat（R11）が documentation-only mitigation。
- **shadow observability（受容）**: `.devin/hooks.v1.json` が
  config.json を precedence で shadow し own entry を実質 inert に
  しうる — `shadow` finding は informational のまま維持し、観察は
  するが gate へは戻さない。
- **conflict の見落とし（受容）**: blocker でなくなり exit code に
  現れないため、consumer が warning を見落とすリスクが残る。
  `warning`・finding・note の文言と docs で緩和する（ADR
  Consequences と同じ評価）。
- **wrapper 経路の warning 不可視（受容）**: `install-devin-hooks.ps1`
  は成功時に sync envelope を echo しないため、`warning` は
  wrapper 利用者に見えない。spec scope 外であり、`verify` が
  観察経路を提供する — residual risk として記録する。

## Deferred Design Concerns

spec の Deferred Design Concerns を引き継ぐ（新規 doc 未作成のため
`defers` relation は張らない）:

- informational finding の構造化（severity field 等）— 現行は
  string finding + `clean` flag で足りており追加構造は YAGNI。
  findings を機械消費する consumer が severity 区別を要求した時点
  で再検討。
- legacy file 側の登録を cheaper 側から収束させる仕組み —
  所有境界上 auto-cleanup は却下済み。Devin 側 import 制御や明示的
  cleanup command の要求が上がった時点で再検討。
- 旧 gate 方針の ADR-034H2WBNtC4We8dyC5lQZJ との関係整理 —
  ADR-034IKJq6tKWWAep0Q0ONzK が `related` link で amend 関係を
  記録済み（本 design での追加作業なし）。

## Verification Notes

- [ ] spec の全 Acceptance Criteria が test design の各項目へ
      対応付けられること（rewrite :132、verify conflict-only
      clean、post-install/post-update 収束、post-uninstall 判定不能
      passthrough、`--quiet` field 持続、retained errors、remove
      note、mjs 書き換え、docs 2 file）。
- [ ] finding・`warning`・`note`・`remediation` の文言に
      "duplicate firing is unsafe" が残っていないこと。
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
      integration hook_config` が pass すること。
- [ ] `node --test scripts/test/devin-hooks-install.test.mjs`
      （Windows + built CLI 環境）が pass すること。
- [ ] `mise run check` が pass すること。

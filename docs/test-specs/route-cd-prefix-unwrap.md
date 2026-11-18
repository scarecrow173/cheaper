---
id: "TSPEC-034sgJmI7QQzZ1S9NbESEe"
type: "test-spec"
status: "draft"
title: "cd prefix unwrap — hook route arm と route run の subdir cwd 検証"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source: []
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
    - "docs/test-specs/hook-route-rewrite.md"
    - "docs/test-specs/route-run-delegated-exec.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cd prefix unwrap — hook route arm と route run の subdir cwd 検証

design R41 の検証 spec。`cd <literal> && <routable>`(および `;`/`\n`
結合)の unwrap が受理形だけに rewrite を発行し、routed child が
cd 移動先 cwd で実行されること、非受理形が fail-open であることを
固定する。

## 受理 shape(rewrite 発行)

- `cd sub && python check.py` — `&&` 結合
- `cd sub; python check.py` / `cd sub\npython check.py` — `;`/`\n` の
  `ShellSeparator::Seq` 結合
- `cd "sub dir" && cmd` — quote 付き literal
- `cd . && cmd` — 自己参照
- cd 無しの単一 safe segment — 従来どおり request cwd で解決

rewrite の payload は `command` = cd prefix 付き原文、
`argv` = cmd segment 由来、`cwd` = canonical 済み移動先
(`payload_root ⊆ supply.root`、同一を含む)。

## 非受理 shape(fail-open — rewrite 不発行、deny にもしない)

- `cd sub || cmd` / `cd sub | cmd` / `cd sub & cmd` — Or/Pipe/
  Background 結合(cd 失敗時 cwd・stdin semantics が変わるため)
- `cd a && b && c` — 3+ segment
- `cd` 単独 / `cd /d sub && cmd` / `cd "" && cmd` — cd segment が
  ちょうど 2 token の `cd` + literal でない
- `CD sub && cmd` — command name は完全一致 `cd` のみ
- `pushd sub && cmd` / `FOO=1 cd sub && cmd` — cd 以外の先行 segment・
  env-prefix 付き cd
- `cd $HOME && cmd` / `cd * && cmd` — expansion・glob を含む unsafe token
- unterminated quote、unsafe segment 含有
- 移動先が不存在・file・canonical request cwd 外部(`cd ..`、`cd /abs`)

## 実行検証

- `route run` は `payload_root ⊆ supply.root` を containment 検証し、
  外部を `cwd-mismatch` で reject(equality-only の旧契約からの緩和)。
- `exec_cwd = payload_root` が子 process cwd へ渡り、routed child が
  cd 移動先で実行される(`WorkflowContext` root・route 層解決・
  usage root は従来どおり workspace root)。
- emitted command は `… route run --payload` のみ — `cd` を残さない
  (`route run` 自身の cwd が subdir になると `.cheaper` 層解決が破れる)。
- `route run` revalidate は `payload.command` を同一の
  `routed_command_segment` 規約で再 tokenize し、cmd segment 由来の
  argv が payload `argv` と一致することを要求 — cd literal と
  `payload.cwd` の一致自体は再検証しない(payload は self-asserted、
  containment が security boundary)。

## 実装 coverage

- `hook::routed_shell_cd_prefix_rewrite_carries_subdir_cwd` ほか
  routing_policy 側の受理/非受理 matrix
- `route_run::e2e_cd_prefix_unwrap_runs_child_in_subdir` — emitted
  payload 経由で child cwd が subdir になる end-to-end
- `route_run::e2e_cd_prefix_fail_open_emits_no_rewrite` — 非受理形が
  rewrite を発行しないこと
- `route_run` 側の containment boundary(payload cwd 偽造で workspace
  外へ出られないこと)

---
id: "TSPEC-034GtAElEShJ9SxImZE2ot"
type: "test-spec"
status: "approved"
title: "APM-distributed Devin registration preservation"
created: "2026-10-15"
updated: "2026-10-15"
owners: ["scarecrow173"]
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
    - "../plans/implement-native-tool-normalization-and-devin-bootstrap.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/case-insensitive-native-tools-and-devin-registration.md"
    - "docs/designs/devin-native-hooks-and-apm-bootstrap.md"
    - "docs/adr/apm-distributed-devin-native-hook-bootstrap.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# APM-distributed Devin registration preservation

## Purpose

T2（APM 配布 `install-devin-hooks.ps1` による `.devin/config.json`
native 登録）の非破壊・冪等・preview/apply・capability・保全
contract を固定する test-spec。consumer の既存 config/hook を
失わないことを最優先で検証する。

## Feature

`install-devin-hooks.ps1`（Windows PowerShell 5.1）による
preview 既定・`-Apply` 限定の Devin native `PreToolUse` read/grep
登録と、書込み前の全保全検査。

## Rules

- Rule: default は preview のみ。file/directory を一切作らず変更概要
  のみ表示し、既存 config 値・raw stdin・capability 生出力を
  ログしない。CLI 不在でも preview 可能。
- Rule: `-Apply` は `cheaper hook targets --json` が exit 0・
  JSON object・`schemaVersion` が integer 1・`targets` が string
  array で exact `devin`（case-sensitive `-ccontains`）を含むとき
  のみ書込む。CLI 不在・旧 binary・nonzero・empty/malformed・
  unknown schema・`devin` 欠落・`'Devin'`（case 違い）は非破壊
  error + Cargo install/update 案内。version string 判定禁止。
- Rule: canonical entry `{matcher:'^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp])$', hooks:[{type:'command',command:'cheaper hook pre-tool-use --target devin'}]}` を `hooks.PreToolUse` へ追加。同一 command+matcher は case-sensitive 比較で no-op（2 回目以降 bytes まで不変）、own command の複数/異 matcher/追加 attributes/改変は非破壊 error。
- Rule: 既存 config の未知 key/値/構造は semantic 保持（object/
  array/null/deep data 不変）。初回 write の UTF-8 format 正規化は
  許容するが、`Get-JsonShape` roundtrip が差異・失敗なら書かず
  保全 error（duplicate key・escaped duplicate・precision loss・
  case collision・等価でも差異となる指数表記を検出/保守的拒否）。
- Rule: root/`.devin`/`config.json` の reparse・symlink・schema 異常
  （top object/hooks object/PreToolUse array/group object 違反）・
  parse 不能は書込みなし error。
- Rule: 既知 project config の限定 scan で cheaper 互換 read/他
  target 重複を検出したら apply 停止 + 相対 config path 通知
  （削除・provider 無効化なし）。shell-only legacy
  `pre-run-command` は非 conflict。unparseable 対象も非破壊 error。
- Rule: `.devin/hooks.v1.json` を生成しない。global/user 設定・
  source 環境を変更しない。書込みは temp + atomic
  `File.Replace`/`File.Move`、書込み直前に原 bytes 等価
  （Base64 比較）+ reparse 再検査、`finally` で自作 temp のみ
  cleanup。

## Examples

- fixture consumer で preview → file 無生成・概要のみ。
- apply → entry 追加・既存 deep data 保持・再 apply は byte 同一
  no-op。
- `.windsurf/hooks.json` に cheaper read hook がある fixture →
  apply 停止 + path 通知。shell-only `pre-run-command` のみなら
  apply 継続。
- capability stub が `{"schemaVersion":1,"targets":[...devin欠落]}`・
  `'Devin'`・malformed JSON・nonzero を返す fixture → 書込みなし
  error。capability 応答由来の secret sentinel を stderr/stdout/書込み
  file に出さない（元 config の値は別途保全する）。

## Guarantees

- 失敗・拒否の全経路で既存 config bytes が不変。
- 登録は APM lockfile 外の補助登録として notice され、APM
  uninstall 後の手動解除と `/hooks` 確認が案内される。

## Non-goals

- global/ancestor hook の完全検出（ユーザーへ `/hooks` 確認を促す）。
- POSIX installer、native exec/shell 登録、consumer 設定の自動移行。

## Risk

- 本 spec の欠落は「通常の導入で黙って guard 無効・他 config 消失」
  という unacceptable failure の見逃しになる。

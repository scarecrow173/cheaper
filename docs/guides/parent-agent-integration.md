# Parent-agent 連携ガイド

`cheaper` は親 Coding Agent（Claude Code / Codex / Copilot /
Cursor / Windsurf 等）から処理を委譲するための worker runtime。
本 guide は親 agent と cheaper の責務分担・配布 primitive・
target 別挙動・検証方法を説明する。

## 役割分担

- **親 Coding Agent**: 判断・統合・ユーザーとの対話。task の
  定義・worker の選定・結果の解釈・workspace への最終統合を行う。
- **cheaper**: 機械的・大 context の処理の実行。worker 定義・
  tool 制約・model 解決・schema 検証・usage 計測を行い、
  構造化された result と meta を返す。

親 agent は `cheaper` を 1-shot CLI として呼び出し、stdin に
task JSON を渡して stdout の envelope を読む（詳細は
[usage ガイド](./usage.md)）。

## 配布 primitive

cheaper は parent agent へ以下を配布する（APM package 経由）:

- **skills**（4 件）:
  - `cheaper-usage`: CLI contract（command・envelope・exit code・
    provider/model 設定）を agent へ教える
  - `cheaper-bulk-read` / `cheaper-code-write` /
    `cheaper-doc-write`: 各 built-in worker への委譲方法
- **hooks**（`pre_read_code`）: read 前に `count-tokens` を実行し、
  threshold（default 8192 token、
  `CHEAPER_READ_TOKEN_THRESHOLD` で上書き）を超える file の
  read を block して `bulk-read` へ誘導する。malformed payload・
  file 不存在・非 file・workspace 外・CLI 起動失敗・出力の
  parse 失敗といった infrastructure failure は fail-open
  （素通し）なので、設定が壊れていても通常の read が止まらない。
  一方、`count-tokens` が parseable な `FILE_TOO_LARGE` envelope
  を返した場合は「計測不能だが確実に巨大」と authoritative に
  判定して fail-closed で block する（Windsurf: exit 2、
  Copilot: `permissionDecision: "deny"` + exit 0 —
  SPEC-034r05iHK8bA7gjwZTji8t /
  DESIGN-034r05iHy2gdnoLt52txFG の意図的仕様）。
- **bundled CLI**: self-contained の `cheaper` CLI（skills/hooks
  が内部から直接呼べる）

## install

consumer（親 agent 環境）は `cheaper` の distributable APM package
を依存として install する:

```yaml
# consumer の apm.yml
dependencies:
  apm:
  - scarecrow173/cheaper/packages/apm#main   # virtual package 参照
  # または local path: - path: ../cheaper/packages/apm
```

```bash
apm install --target windsurf   # 利用する target を指定
```

## target 別挙動

hook の動作は target の hook event 体系に依存する。

- **Windsurf**: `pre_read_code` event が native で、read 前に
  hook が実行される。threshold 超過 file は block + bulk-read
  への誘導メッセージが返る。
- **Copilot**: `preToolUse` event 経由で adapter
  （`pre-read-code-copilot.js`）が動作する。Copilot では非 0
  exit が deny 扱いになるため、adapter は常に exit 0 で終了し、
  threshold 超過または `FILE_TOO_LARGE` verdict の `view` tool
  呼び出しだけを stdout の
  `{"permissionDecision":"deny"}` で block する（ADR-034r05iHrZUE6WizQ4LgPa）。
  既知の制限として、現行の APM は copilot target へ `.json`
  asset を deploy しないため、file-schema worker（bulk-read /
  code-write / doc-write）は upstream 修正待ちの間利用できない
  （`count-tokens` と hook adapter は動作する）。
- **その他 target**（Claude / Codex / Cursor 等）: hook 登録は
  deploy されるが、`pre_read_code` event を発火しない。
  Claude では `preToolUse` 登録が `PreToolUse` に正規化されて
  発火し得るが、stdin shape が adapter の前提と異なるため常に
  素通し（exit 0）となり、いずれも no-op として動作する
  （登録は harmless、機能は無効）。target 固有の adapter 化は
  今後の拡張余地（ADR-034r05iHrZUE6W9RDF0V73・DISC-034r05iHy2gdnpdai9KGpW）。

skills（`cheaper-usage` 等）は target 非依存で機能する。

## 自動 vs 明示的な委譲

- **自動**: hook（`pre_read_code`）が threshold 超過 read を
  検出して parent agent へ bulk-read 委譲を促す。
- **明示**: parent agent（またはユーザーが skill を発動）が
  `cheaper bulk-read` 等を直接呼ぶ。

## 閾値の設定

`CHEAPER_READ_TOKEN_THRESHOLD` で large-read threshold（default
8192 token）を変更できる。hook の判定は workspace 内 file に
対する `count-tokens` の結果 envelope のみに依存する。
workspace 外・不存在・非 file・CLI 起動失敗・出力の parse 失敗
などの infrastructure failure は fail-open（素通し）で、
parseable な `FILE_TOO_LARGE` envelope は fail-closed で
block される。

## 連携の確認

```bash
# hook bundle に deploy された CLI の動作確認
<deployed-hook>/cli/bin/cheaper.mjs --version
<deployed-hook>/cli/bin/cheaper.mjs count-tokens <file>

# skills/hooks が install されたかは target の config を確認
# （例: .windsurf/hooks/ 等、target 固有の deploy path）
```

実際の e2e 検証手順は [CONTRIBUTING.md](../../CONTRIBUTING.md) の
「実 provider 検証」を参照。

## 関連

- CLI contract: [usage ガイド](./usage.md)
- provider / auth 設定: [providers ガイド](./providers.md)
- 設計記録: `docs/designs/apm-skills-and-pre-read-hook.md`・
  `docs/adr/distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md`・
  `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`

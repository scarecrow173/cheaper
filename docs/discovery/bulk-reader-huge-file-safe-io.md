---
id: "DISC-034r05iHy2gdnpeCjN4yfM"
type: "discovery"
status: "resolved"
title: "bulk-reader の巨大ファイル安全 I/O"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../../crates/cheaper-tools/src/tools/boundary.rs"
    - "../../crates/cheaper-tools/src/registry.rs"
    - "../../crates/cheaper-tools/src/walker.rs"
    - "../../crates/cheaper-tools/src/tools/grep.rs"
    - "../../crates/cheaper-tools/src/tools/find.rs"
    - "../../crates/cheaper-tools/src/tools/count_tokens.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
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
    - "../adr/treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md"
    - "../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
    - "../designs/huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "../designs/workspace-filesystem-boundary-sandboxed-tools.md"
    - "../designs/bulk-reader-contract.md"
    - "./cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from:
    - "./huge-file-token-count-guard-and-fail-closed-hook-behavior.md"
  derived-by:
    - "../specs/bulk-reader-huge-file-safe-io.md"
    - "../adr/bulk-reader-huge-file-safe-io.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# bulk-reader の巨大ファイル安全 I/O

## Exploration Goal

100MB 以上の巨大ファイルを bulk-reader に委譲したとき、tool 層がメモリを使い切ったり、検索対象を黙って見落としたりしないための I/O 方式と結果契約を探索する。DISC-034r05iHy2gdnpcXzmUKaV → SPEC-034r05iHK8bA7gjwZTji8t / ADR-034r05iHrZUE6W82VsDbPq → DESIGN-034r05iHy2gdnoLt52txFG で実装された token count の 16MB byte cap と fail-closed hook の後続課題である。ADR-034r05iHrZUE6W82VsDbPq の Consequences が明記する、委譲先そのものの巨大ファイル対応を扱う。

本書は coordinator による現行コード確認と dispatch 時の探索範囲を入力にした A-3（Combined Discovery）の探索記録である。個別に指定された文書作成タスクとして briefing-flow の SUBAGENT-STOP に従い、方式の採用や要件の承認は行わない。100MB 以上は探索対象の規模であり、新しい上限値の決定ではない。

## Key Issues and Assumptions

### 確認済みの現状

| 対象 | 現行の動作と根拠 | 巨大ファイルでの問題 |
| --- | --- | --- |
| read | `operations.ts` の `createReadOperations.readFile` はファイル全体を `readFile` して Buffer を返す。`registry.ts` は Pi の `createReadToolDefinition` を使用する | Pi の `ReadOperations` は `readFile -> Buffer` 契約のため、offset/limit による切り詰め前に全体がメモリに載る。出力制限だけではメモリ使用量を制限できない |
| walker | `walker.ts` の `MAX_FILE_BYTES` は 1MB。超過ファイルと先頭 8KiB の NUL 検査で binary と判定したファイルを `isEligibleFile` が除外する | `walkFiles` を共有する grep と find の両方から巨大ファイルが黙って消える |
| grep | 適格ファイルを `readFile(real, 'utf8')` で全体読み込みする。正規表現評価は worker_thread に隔離され、per-file 5秒 / total 60秒の watchdog がある | 現状は 1MB 以下しか到達しない。walker のサイズ制限だけを外すと全体読み込みのリスクが拡大する。正規表現の隔離は I/O のメモリ上限を保証しない |
| find | display path に対する glob 一致を調べ、内容を読まない | 内容を使わない検索にもサイズ・binary 判定が適用され、巨大ファイルを列挙できない |
| count_tokens | core の `countFileTokens` を呼び、`FileTooLargeError` は tool error として伝播する | 16MB cap は維持する前提。安全な部分読み込みができても巨大ファイル全体の canonical token count が可能になるわけではない |
| parent hook | `cheaper hook`（`packages/cli/src/hook/`。旧 `.apm/hooks/pre-read-code.js`）は閾値超過または `FILE_TOO_LARGE` で直接 Read を deny と委譲 guidance によりブロックする | parent の保護は既存対応済みだが、委譲先 bulk-reader の安全性・検索の完全性は別問題として残る |

Pi の契約確認先は `packages/runtime-pi/node_modules/@earendil-works/pi-coding-agent/dist/core/tools/read.d.ts`。これはインストール済み依存の型定義であり、永続的なリポジトリ内リンクではない。

### 制約と探索境界

- workspace 境界、realpath によるパス検証、キャンセル、既存の正規表現隔離を維持する観点で比較する（DESIGN-034r05iHrZUE6WsgJOy6Ny）。
- bulk-reader の出力契約との整合を確認する（DESIGN-034r05iHrZUE6WxkSJi9GF）。部分的な結果を完全な探索結果と誤認させないことが論点となる。
- core の byte cap、canonical tokenizer、既存 fail-closed hook の再設計は対象外。DISC-034r05iHy2gdnpdai9KGpW の agent 別 hook 配布・契約マトリクスとも責務を分ける。
- メモリ、読み込み byte 数、経過時間、返却量は別々の予算として考える。具体的な上限値、設定方法、互換性は未決である。

## Alternatives and Trade-offs

| 論点 | 選択肢 | 利点 | 制約・トレードオフ |
| --- | --- | --- | --- |
| read | sandboxed worker 向けの独自 bounded read：`FileHandle.read` による範囲読み込み、または行 window と per-call byte cap | 全体 Buffer 化を避け、読み取り量と返却量を明示的に扱える | Pi read の表示・offset/limit・切り詰め通知との互換性、UTF-8 境界、行番号の意味を定義する必要がある。行 offset を探す走査時間も制限対象になりうる |
| read | Pi read を維持 | 既存実装・出力形式を利用できる | 現行の whole-Buffer operations 契約では巨大ファイルの bounded I/O に不適合。offset/limit の指定だけでは解決せず、別の拒否・制限契約が必要 |
| grep | `createReadStream` + `readline` の行単位 streaming | ファイルサイズによる一律除外を避け、通常の短い行ではメモリを抑えられる | 改行なしの巨大な単一行は `readline` だけでは bounded memory にならない。最大行長、打ち切り、worker_thread への受け渡し単位を検討する必要がある |
| grep | chunk 単位読み込み | 固定 byte 予算を設定しやすく、巨大行も扱う余地がある | UTF-8・行・正規表現の境界をまたぐ一致に注意が必要。任意 regex に有限 overlap だけで完全性を保証できるとは限らない |
| grep | サイズ制限を維持し、skip を明示 | 現行のメモリ保護を維持しながら見落としを可視化できる | 巨大ファイル内の検索はできない。委譲要求に対する未探索範囲が残る |
| find | サイズ・binary eligibility を外し、path のみで一致判定 | 巨大ファイルや binary を列挙でき、内容判定の I/O を省ける | grep と walker の責務を分ける必要がある。列挙件数・時間の予算や従来結果との差を検討する必要がある |
| find | 現行 eligibility を維持 | grep と対象集合を揃え、現行動作を保てる | path 検索に無関係な理由でファイルを見落とす。維持するなら制限を明示する必要がある |
| skip 通知 | binary / unreadable / oversized / 予算超過を理由付きで報告 | 空結果と未探索を区別でき、親 agent が追跡できる | 大量 skip 時に通知自体が膨らむ。件数・代表例・残件数などの上限付き表現を検討する必要がある |
| skip 通知 | 現行の silent skip を維持 | 出力が小さく、既存の結果形状を保てる | 「一致なし」と「調べていない」を区別できず、完全性を過大評価しうる |
| read の binary | bounded read に binary guard を追加 | binary の無意味なテキスト出力を抑える | NUL sniff の誤判定、UTF-16 等の扱い、拒否・例外許可の契約が必要 |
| read の binary | binary guard を追加せず byte cap のみ | 現行 read の受け付け範囲を保てる | メモリは制限できても文字化けや役に立たない出力は残る |

### 時間予算と完全性

grep の既存 `evalTotalMs` watchdog と、新しい I/O・decode・行分割・worker 転送の時間予算との関係を調べる必要がある。per-file 5秒 / total 60秒をそのまま用いても、新しい走査経路の経過時間全体が制限されるとは未確認である。byte または時間予算で中断した際に、既得の一致を返すか、エラーにするか、未探索範囲をどう示すかは未決とする。

## Tentative Conclusions

- 仮説：read の安全性を改善するには、表示の切り詰めより前の I/O 境界で読み取り量を制限する必要がある。独自 read の方式・契約はまだ選定しない。
- 仮説：path 列挙と内容検索を別々の適格性で扱うと、find の欠落を解消しつつ grep の保護を検討しやすい。ただし walker の分離方式は未決である。
- 仮説：安全な打ち切りと未探索範囲の可視化を合わせて設計すると、親 agent が追加探索の必要性を判断しやすくなる。
- これらはコード上の構造からの仮説であり、100MB 以上での peak memory・所要時間の測定結果ではない。本探索で実装変更や性能検証は行っていない。

## Open Questions

- **実装前の決定事項**：read の byte range / 行 window の選択、offset/limit の互換性、per-call byte cap、途中終了・再開位置の表現をどうするか。
- **実装前の決定事項**：grep の streaming / chunk / skip 報告のどれを採用し、単一巨大行・境界をまたぐ一致・正規表現評価の隔離をどう扱うか。
- **実装前の決定事項**：grep の per-file byte/time budget と `evalTotalMs` をどう統合し、予算超過を部分結果とエラーのどちらとして扱うか。
- **実装前の決定事項**：find の size/binary eligibility を外すか。除外する場合、workspace 検証を共有しつつ内容判定だけを分ける境界はどこか。
- **実装前の決定事項**：skip 理由を tool 出力と bulk-reader の結果契約にどう表現し、通知量をどう制限するか。read に binary guard を設けるか。
- **後続の検証設計事項**：100MB 以上の通常テキスト、改行なしの巨大行、minified bundle、binary、unreadable、読み込み中の増大・変更に対して、メモリ・時間・結果の完全性をどう観測するか。
- **後続の検証設計事項**：UTF-8 境界、キャンセル、ReDoS、workspace 外 symlink を含む既存保護の回帰をどのテストで示すか。

## Promotion Candidates

- [x] spec-doc：[SPEC-034r05iHK8bA7gkTEj7TdU](../specs/bulk-reader-huge-file-safe-io.md) として作成済み。巨大ファイル read / grep / find の期待動作、予算超過・skip・部分結果の利用者向け契約と受け入れ条件。
- [x] adr-doc：[ADR-034r05iHrZUE6Wb2K7GmPw](../adr/bulk-reader-huge-file-safe-io.md) として作成済み。bounded I/O 方式、Pi read との差し替え可否、walker の責務、grep の評価隔離と時間予算の選択理由。
- [ ] design-doc：選定後の tool API、workspace 境界を含む I/O フロー、DESIGN-034r05iHrZUE6WsgJOy6Ny / DESIGN-034r05iHrZUE6WxkSJi9GF への影響、検証方法。

spec と ADR は本書の後に作成され、`derived-by` に記録した。本書は draft とする。

## Sources

- 調査入力：2026-09-17 の coordinator dispatch に含まれる現行コード確認結果。上記 `relations.source` のファイルと Pi の型定義を根拠として受領した。独立した再実装・実測は行っていない。
- [DISC-034r05iHy2gdnpcXzmUKaV](./huge-file-token-count-guard-and-fail-closed-hook-behavior.md)：token count と fail-closed の先行探索。委譲先 I/O は scope 外として残された。
- [ADR-034r05iHrZUE6W82VsDbPq](../adr/treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md)：16MB cap と委譲先 I/O の follow-up を明記。
- [SPEC-034r05iHK8bA7gjwZTji8t](../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md) / [DESIGN-034r05iHy2gdnoLt52txFG](../designs/huge-file-byte-cap-guard-and-fail-closed-hook.md)：先行する token count・hook の要求と設計。
- [DESIGN-034r05iHrZUE6WsgJOy6Ny](../designs/workspace-filesystem-boundary-sandboxed-tools.md) / [DESIGN-034r05iHrZUE6WxkSJi9GF](../designs/bulk-reader-contract.md)：sandboxed tools と bulk-reader 契約の関連文書。
- [DISC-034r05iHy2gdnpdai9KGpW](./cross-agent-hook-distribution-and-contract-matrix.md)：agent 別 hook 配布・契約の関連探索。

既存文書との関係では、DISC-034r05iHy2gdnpcXzmUKaV の巨大ファイル委譲先対応が scope 外という記述と、ADR-034r05iHrZUE6W82VsDbPq の follow-up 記述は整合する。先行 chain の完了を bulk-reader 全体の巨大ファイル対応済みと解釈しないよう、本書で残る境界を明示する。

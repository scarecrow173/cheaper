---
id: "ADR-0017"
type: "adr"
status: "proposed"
title: "bulk-reader の巨大ファイル I/O を有界 read・streaming grep・path-only find にする"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0010-bulk-reader-huge-file-safe-io.md"
    - "../specs/0014-bulk-reader-huge-file-safe-io.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0015-treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md"
    - "../designs/0002-workspace-filesystem-boundary-sandboxed-tools.md"
    - "../designs/0006-bulk-reader-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0010-bulk-reader-huge-file-safe-io.md"
    - "../specs/0014-bulk-reader-huge-file-safe-io.md"
  derived-by:
    - "../designs/0019-bulk-reader-huge-file-safe-io.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 17. bulk-reader の巨大ファイル I/O を有界 read・streaming grep・path-only find にする

## Context and Problem Statement

ADR-0015 は token count の既定 16MB cap と親 Read の fail-closed を定めたが、
Consequences に委譲先 bulk-reader の巨大ファイル対応を後続課題として残した。
100MB 以上の入力を親から退避しても、worker が全文を保持するか黙って検索対象から
除外するなら、安全な委譲は完結しない。

確認済みの現状は次のとおりである。

- `registry.ts` は read に Pi 0.85.1 の `createReadToolDefinition` を使う。
  `ReadOperations.readFile` は `Promise<Buffer>` でファイル全体を返す契約であり、
  Pi の `DEFAULT_MAX_BYTES` / `DEFAULT_MAX_LINES` と truncation は読み込み後に働く。
  offset/limit を指定しても元の Buffer の大きさは制限されない。
- `walker.ts` は `MAX_FILE_BYTES = 1MB` と先頭 8KiB の NUL sniff によりファイルを
  黙って除外する。grep と find が共有しており、パスだけを使う find にも影響する。
- `grep.ts` は `readFile(..., "utf8")` で全文を保持し、全文を評価 worker に送る。
  worker は行単位で JavaScript 正規表現を評価する。既存の評価制限は per-file
  5 秒・累積 60 秒、結果件数は既定 100・最大 500 である。
- `count_tokens` の `FileTooLargeError` と既定 16MB cap は意図された保護である。
  `EditOperations.readFile` も全文を扱うが、編集方式の変更は本 ADR の対象外である。

ここでいう有界性は、入力ファイルの総 byte 数を N として、同じ設定での作業メモリを
O(N) にしないことを指す。出力だけの truncation、`highWaterMark` だけの指定、
`readline` だけの導入ではこの条件を満たさない。改行のない 100MB の単一行にも適用する。

## Decision Drivers

- ファイルサイズと独立した buffer・出力・待機キューの上限を持つ。
- 巨大ファイルの必要範囲を読み、後半の一致とパスを発見できる。
- 未検索と一致なし、部分読み取りと EOF を明確に区別する。
- KISS に従い、既存の `defineTool`、Node.js I/O、評価 worker を活用する。
- 外部プロセスを spawn せず、workspace boundary・symlink policy・privacy deny を維持する。
- `count_tokens` の 16MB cap、親 hook、edit、write-scope、bulk-reader の出力 schema と
  excerpt 制約は変更しない。

## Considered Options

- **Option 1**: read を独自の bounded `defineTool` に置き換える。
- **Option 2**: Pi read を維持し、注入する operations だけを工夫する。
- **Option 3**: grep を有界な行分割と batch による streaming 評価へ移す。
- **Option 4**: grep を任意の byte chunk 単位で評価する。
- **Option 5**: grep の巨大ファイル除外を維持し、除外だけを通知する。
- **Option 6**: find を path-only にする、または既存の size/binary eligibility を維持する。
- **Option 7**: 除外理由を有界な footer で通知する、または silent skip を維持する。

## Decision Outcome

提案する組み合わせは **1 + 3 + path-only find + 有界な除外通知** である。
Pi の全文 Buffer 契約を外し、内容検索にだけ text eligibility を適用することで、
有界性と巨大ファイルの可視性を同時に満たせる。

方式の比較はこの組み合わせを支持する。ただし単一長大行の制限と read の継続 API、
Pi read の画像機能との互換性は利用者に見える変更である。これらの詳細な契約と予算を
後続設計で確定するため、本記録は `proposed` とする。SPEC-0014 の数値未確定部分を
承認済みと扱わず、実装完了も意味しない。

### read の提案契約

`registry.ts` の sandboxed read を独自 `defineTool` に置き換え、境界検証済みの
FileHandle に対する固定上限付き `read` を使う。既存の `path` と行ベースの
`offset` / `limit` の利用を維持する方向とし、目的行までの prefix は固定 buffer で
走査して破棄する。行を丸ごと蓄積してから切り詰めない。

返却 byte cap を満たした時点で部分結果と truncation 理由を返す。EOF、行数上限、
byte 上限を区別し、巨大な単一行も後続を取得できる byte 単位の継続情報を設ける。
通常の行 offset と byte 継続位置は混同せず、同時指定の扱い、位置の起点、UTF-8 の
文字境界と不正 byte の扱いを schema と設計に明記する。継続読み取りは snapshot
保証ではなく、途中でファイルが変更された場合の限界も通知仕様に含める。

画像・binary の既存 Pi read 動作を無条件で引き継ぐとは約束しない。非対応入力は
明示的に拒否し、全文読み込みへの fallback は設けない。画像対応を残すなら同じ
有界性を証明できる別の契約が必要であり、本提案の既定経路には含めない。

### grep の提案契約

固定サイズの `createReadStream` または FileHandle 読み取りから、上限付きの
行フレーマーで行を構築する。単純な `createReadStream + readline` は、一行が
無制限に伸びるため採用しない。UTF-8 と CRLF の境界状態を保持し、chunk をまたぐ
通常行を完全に復元してから評価する。検索意味論は既存どおり行単位であり、任意の
複数行正規表現への拡張は行わない。

現在の全文 `file` メッセージを、byte 数と行数の双方に上限を持つ batch protocol
へ置き換える。同時に未応答の batch は 1 つに制限し、応答後に次を送ることで
backpressure をかける。正規表現は引き続き worker 内で評価し、main thread では
評価しない。context は最大 3 行の前後を扱う有限 ring buffer と pending context
で保持し、重複 context を統合する。全行配列や全一致位置配列を作らない。

一行の byte 上限を超えた場合は、その行を評価せず newline まで破棄して走査を
継続し、対象パス・行番号・理由を部分結果の footer に示す。巨大ファイルそのものは
除外しない。長大行を黙って分割して正規表現を適用すると `^` / `$` や境界をまたぐ
一致の意味が変わるため、行全体を評価したと偽らない。対象行の内容確認には bounded
read の継続を案内する。この制限を受け入れることが acceptance 前の確認事項となる。

per-file 5 秒・全体 60 秒の評価予算は batch ごとにリセットせず累積する。
catastrophic backtracking は既存同様に worker を terminate して停止する。
これとは別に I/O・行分割を含む走査の deadline と cancellation を設け、その数値と
既存評価予算との関係を後続設計で決める。終了時は stream / FileHandle / worker を
必ず閉じ、timeout や中断を完全な結果として返さない。

### find と除外通知の提案契約

walker は許可されたパスの列挙に専念し、1MB cap と binary sniff を共有 eligibility
から外す。find はサイズ・内容に関係なくパスを glob に照合し、内容を open/read
しない。`.git` / `node_modules` の既存除外、symlink policy、denylist、結果上限は
維持する。text 判定の bounded sniff は grep 側に置く。

grep の binary 判定、許可されたファイルの unreadable / I/O error、長大行、
timeout、結果上限を tool output の footer に理由別に報告する。find でも列挙失敗
を通知する。代表パス・行番号の件数と表示 byte 数を制限し、残りは集計と通知自体の
省略表示にまとめる。パス文字列の長さにも上限を適用する。

privacy deny 対象は結果・通知・件数に一切含めない。存在秘匿を優先し、deny 対象の
件数を集計して漏らす変更も禁止する。直接指定したパスの境界違反や policy 拒否は
既存の error 契約を維持し、普通の unreadable として飲み込まない。

### メモリ予算

後続設計では I/O chunk C、read 出力 R、最大行 L、batch byte 数 B と行数 K、
grep 出力 O、footer S を共有の定義箇所に置く。すべて入力の N と独立した有限値とし、
呼び出し引数で無制限に引き上げられないようにする。

grep の追加保持量は O(C + L + B + context × L + O + S) とし、worker への複製、
UTF-8 decode 後の文字列表現、結果 formatting の一時コピーも予算に含める。
worker の正規表現用メモリは L と pattern 上限、必要な worker resource limits を
含めて評価する。watchdog は時間制限であり、単独でメモリ上限の証明にはならない。
複数 tool call を並行実行する場合はその最大数を掛けた予算を明示する。

### Consequences

- Good: 100MB 以上の text を範囲読み取り・検索でき、find が巨大/binary file を見落とさなくなる。
- Good: 未処理理由が可視化され、親と worker が完全性を判断できる。
- Good: 外部プロセスと追加ライブラリを必要とせず、sandboxed tool の構成を維持できる。
- Bad: read の互換 API、grep の batch protocol、context の境界処理を実装する責務が増える。
- Bad: 長大な単一行は grep の完全検索を保証しない。明示した制限と bounded read で対処する。
- Bad: 深い行 offset の read は O(N) の走査時間を要し得る。メモリ有界性は高速性の保証ではない。
- Follow-up: 上記 API・上限値・deadline・互換性を design に確定し、SPEC-0014 と整合させて acceptance を判断する。
- Follow-up: 実装 task と regression 検証を作成し、DESIGN-0002 / DESIGN-0006 の旧 read/eligibility 記述を更新する。

### Confirmation

SPEC-0014 の受け入れ条件と下記 Verification を満たすことで実装を確認する。
100MB fixture の結果だけでなく、最大 allocation・保持量・未応答 batch 数を
instrumentation で検証する。RSS の一回の観測だけで有界性を証明しない。

## Implementation Plan

1. **契約確定**: 後続 design に read の範囲・継続 schema、cap の数値、長大行制限、
   時間/メモリ予算、画像互換性を記録する。数値が未確定のまま実装着手しない。
2. **列挙の分離**: `packages/runtime-pi/src/tools/walker.ts` と `find.ts` で
   path enumeration と text eligibility を分離し、上限付きの列挙失敗通知を導入する。
3. **bounded read**: 新規 `packages/runtime-pi/src/tools/read.ts` に独自 tool を
   実装し、`registry.ts` の read mapping を置き換える。`operations.ts` の既存
   boundary / deny 判定と同じ知識を再利用し、検証を迂回する別ルートを作らない。
   edit 用全文操作は変更しない。共通 buffer 上限は一箇所で定義する。
4. **streaming grep**: `packages/runtime-pi/src/tools/grep.ts` の全文読み込みと
   worker protocol を置き換える。必要なら行フレーマーを小さな共通 helper に分ける。
   単純な readline、全 chunk の `Buffer.concat`、未制限の Promise/メッセージ蓄積、
   main thread での regex 評価、外部 `rg` / shell 呼び出しは禁止する。
5. **互換性**: read の共有 registry 変更は bulk-reader 以外にも及ぶため、writer の
   read 利用を確認する。tool description と必要な worker guidance を更新する。
   write-scope・tool budget・拒否計上・出力 schema は維持する。依存追加と新しい env
   設定は既定では不要である。
6. **検証**: `packages/runtime-pi/test/tools/` の `walker.test.ts`、`find.test.ts`、
   `grep.test.ts`、`registry.test.ts` と新規 `read.test.ts` に境界/有界性のテストを置く。
   型チェックと runtime-pi の unit test は `mise exec -- pnpm --filter
   @cheaper/runtime-pi build` / `mise exec -- pnpm --filter @cheaper/runtime-pi test`
   を使用する。実 provider / CLI / hook 検証はリポジトリ外の使い捨てインストール環境
   に `apm install` して行う。本リポジトリ内の cheaper CLI、bundled hook、cheaper-*
   スキルによる自己委譲や検証は実行しない。

## Verification

- [ ] 16MB 超・100MB 以上の fixture を固定サイズで生成し、read/grep が全文
      `readFile` を呼ばず、低層 read 要求・各 buffer・batch・出力が設計 cap 以下である。
- [ ] 改行なしの 100MB 単一行でも allocation と保持量が cap を超えず、read は
      継続位置を返し、grep は長大行の未評価を明示する。
- [ ] 空ファイル、先頭/中間/末尾、EOF、CRLF、UTF-8 の chunk/返却境界、巨大 offset、
      不正範囲、ファイル変更について文書化した結果になる。
- [ ] 100MB text の後半の既知の一致、chunk/batch 境界をまたぐ通常行の一致、
      ignoreCase、glob、最大 3 行の context、重複 context と行番号が正しい。
- [ ] 多数一致でも既定 100・最大 500 と byte 出力 cap を守り、未応答 batch は
      最大 1、skip footer も件数/byte cap 内で省略の事実が分かる。
- [ ] binary、unreadable、途中 I/O error、timeout、abort を注入し、不完全な探索が
      通常の `(no matches)` だけにならず、stream/handle/worker が解放される。
- [ ] ReDoS を worker で停止でき、per-file/total の評価予算を batch ごとにリセットしない。
      走査 deadline と並行 call を含むメモリ予算も設計値に従う。
- [ ] find が 100MB file と binary file を返し、内容の open/read を行わない。
- [ ] workspace escape、symlink、built-in/custom deny の回帰テストが通り、deny の
      パス・存在・件数が結果/skip footer に漏れない。read-only と write-scope が維持される。
- [ ] `count_tokens` の既定 16MB cap と cap 未満の canonical count、親 hook の
      fail-closed を維持する。関連する既存 test と追加 test、型チェックが通る。
- [ ] provider / CLI / hook の検証記録が隔離インストール先を示し、開発中の bundled
      cheaper CLI・hook をこのリポジトリ内で実行していない。

## Pros and Cons of Options

### 1. 独自 bounded read（推奨）

- Good: I/O と保持量を取得前に制限でき、巨大ファイルを小さい範囲で調査できる。
- Good: grep/find と同じ `defineTool` の構成を使える。
- Bad: offset/limit、UTF-8、継続と Pi read 互換性を自前で定義・検証する必要がある。

### 2. Pi read と smarter operations（不採用）

- Good: Pi の UI と truncation 処理をそのまま利用できる。
- Bad: `readFile -> whole-file Buffer` の契約では peak memory を有界にできない。
  部分 Buffer を全文として返すと offset/limit と EOF の意味が壊れる。cap 超過を
  error にするだけなら安全だが、巨大ファイルを調査する要件は満たさない。

### 3. 行単位 streaming と有界 batch（推奨）

- Good: 現行の行単位 JavaScript regex と行番号を保ち、worker 隔離を再利用できる。
- Bad: protocol と context 処理が変わり、単一長大行の制限が必要になる。
  `readline` だけでは未終端行を無制限に保持するため、そのままでは採用できない。

### 4. 任意 byte chunk の regex scan（不採用）

- Good: chunk の保持量は制限しやすく、binary 向けの単純一致にも拡張しやすい。
- Bad: 任意 regex の一致長には有限の上限がなく、固定 overlap で現行の意味論を
  保証できない。UTF-8、アンカー、行番号、context の複雑さも増える。
  固定文字列専用などへの機能縮小は現行 grep と互換でない。

### 5. 巨大ファイル除外と通知（不採用）

- Good: 現行 cap を維持でき、通知により silent skip は解消する。
- Bad: 親から委譲された巨大ファイルの一致を発見できず、ADR-0015 の後続課題を解決しない。

### 6. find の path-only 化（推奨）と eligibility 維持

- Good: path-only 化は不要な content I/O と無関係な除外を取り除く最小の変更である。
- Bad: grep と find の対象集合が異なるので description と既存 test の更新が必要になる。
- 維持案は変更量が小さい一方、巨大/binary file のパスを失う理由がなく、採用しない。

### 7. 有界 footer（推奨）と silent skip

- Good: 有界 footer は結果の完全性を伝え、大量除外でも出力を制限できる。
- Bad: 集計・代表例・存在秘匿の扱いが必要で、footer の分だけ出力が増える。
- silent skip は出力が小さいが、未探索を一致なしと誤認させるため採用しない。

## More Information

- [DISC-0010](../discovery/0010-bulk-reader-huge-file-safe-io.md) と
  [SPEC-0014](../specs/0014-bulk-reader-huge-file-safe-io.md) から導出した。
- [ADR-0015](./0015-treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md)
  の巨大ファイル委譲に関する follow-up を扱う。token count の決定は置き換えない。
- [ADR-0007](./0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md)、
  [DESIGN-0002](../designs/0002-workspace-filesystem-boundary-sandboxed-tools.md)、
  [DESIGN-0006](../designs/0006-bulk-reader-contract.md) が境界と worker 契約を規定する。
- Pi の確認対象はインストール済み 0.85.1 の
  `packages/runtime-pi/node_modules/@earendil-works/pi-coding-agent/dist/core/tools/read.d.ts`
  である。SDK が bounded streaming 契約を提供した場合、または長大行の完全検索が
  必須になった場合に再検討する。

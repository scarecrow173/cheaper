---
id: "SPEC-034r05iHK8bA7gkTEj7TdU"
type: "spec"
status: "implemented"
title: "Bulk-reader huge-file safe I/O"
created: "2026-09-17"
updated: "2026-09-17"
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
  implemented-by:
    - "../designs/bulk-reader-huge-file-safe-io.md"
  depends-on:
    - "./write-scope-and-privacy-deny.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md"
    - "./huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
    - "../designs/workspace-filesystem-boundary-sandboxed-tools.md"
    - "../designs/bulk-reader-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/bulk-reader-huge-file-safe-io.md"
  derived-by:
    - "../adr/bulk-reader-huge-file-safe-io.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "must"
---

# Bulk-reader huge-file safe I/O

## What Should Be Built

bulk-reader の sandboxed `read` / `grep` / `find` が、100MB 以上の
ファイルもメモリ使用量を制限して扱える契約を定める。`read` は範囲指定と
呼び出し単位の byte cap を持ち、`grep` は巨大ファイルを黙って検索対象から
落とさず、`find` はサイズにかかわらず許可されたパスを列挙できる。
処理が不完全な場合は、その事実と理由を呼び出し元へ返す。

## Why Now

SPEC-034r05iHK8bA7gjwZTji8t により、親の直接 Read は `FILE_TOO_LARGE` または token threshold
超過で hook が exit 2 と委譲 guidance を返すようになった。しかし委譲先の
`read` は Pi の `createReadToolDefinition` と `ReadOperations.readFile` により
全体を Buffer に読み込んだ後で offset/limit を適用するため、巨大ファイルで
メモリ使用量が増える。

共有 walker は 1MB 超と先頭 8KiB の NUL 検出による binary を黙って除外する。
そのため `grep` は一致があっても報告せず、内容を必要としない `find` もパスを
返せない。`grep` 自体にも UTF-8 全体読み込みがある。親側の block だけでは
安全な委譲は完結しない。

## Users and Value

- 親 Coding Agent: 巨大ファイルを直接展開せず、委譲先の根拠と処理範囲を判断できる。
- bulk-reader: 必要な範囲を段階的に読み、巨大ファイル中の一致とパスを発見できる。
- 利用者・運用者: 入力サイズに比例したメモリ枯渇と、未検索を一致なしと誤認する結果を防ぐ。

## Scope

### In Scope

- sandboxed read の範囲指定、byte cap、上限到達時の明示的な応答。
- grep の巨大ファイル検索と有界な作業メモリ、find の巨大ファイル可視性。
- binary・読み取り不能・制限到達などによる未処理の報告。
- workspace boundary、privacy deny、write-scope、read-only の既存不変条件の維持。
- 有界 I/O、メモリ、既存動作の回帰検証。

### Out of Scope

- `count_tokens` の既定 16MB cap の引き上げ、撤廃、近似 token count。
- 親の Read hook の block 条件や infra 障害時の fail-open 契約の変更。
- binary の内容検索、bulk-reader の出力 schema・excerpt 上限の拡張。
- このリポジトリ内の bundled cheaper CLI・hook を直接実行する実 provider 検証。

## Requirements

1. **有界 read**: 呼び出し元が指定した範囲を取得でき、範囲省略時も有限の
   byte cap を適用する。全文 Buffer/string を作った後の切り詰めは不可とする。
   巨大な offset、単一の長大行、改行なしのファイルでも作業メモリを有界にする。
   範囲の単位・既定値・cap の数値は設計で明文化し、既存 offset/limit 利用との
   互換性を定義する。返却内容は適用された cap を超えない。
2. **上限と継続**: cap 到達時は tool error または明示的な truncation marker を
   返す。どちらを採用するかは設計で決める。完全な読み取りと区別でき、次の範囲を
   指定するための情報または対処 guidance が得られること。境界での文字欠落・重複や
   UTF-8 の破損を正常な全文として返さない。
3. **巨大ファイルの grep**: サイズだけを理由に 1MB 超の許可された text file を
   検索対象から除外しない。全文を保持せず検索でき、処理境界をまたぐ一致を
   黙って失わない。既存検索オプションの互換性を定義し、制限で評価できない場合は
   明示的な error または部分結果と理由を返す。未検索を「一致なし」と報告しない。
   現行の per-file 5 秒・全体 60 秒 watchdog が担う時間制限を維持し、timeout も
   不完全な結果として識別できること。
4. **find の可視性**: パス検索にファイルサイズや binary 内容による除外を適用
   しない。許可された巨大ファイル・binary file を、内容全体の読み取りなしで
   列挙する。列挙自体が失敗した場合は、その不完全性を報告する。
5. **未処理の報告**: binary 内容の検索除外、読み取り不能、I/O error、timeout、
   出力上限など、処理が完了しなかった理由を返す。許可された対象は対象パスと
   理由を識別できること。報告の件数・サイズにも上限を設け、超過時は報告自体の
   省略を明示する。ただし privacy deny 対象は SPEC-034r05iHK8bA7ggPjqIA9E の存在秘匿を優先し、
   検索結果・skip 報告・件数から存在やパスを漏らさない。
6. **メモリ上限**: read/grep/find の入力処理、デコード、行・一致・パス・skip
   報告の蓄積を含む作業メモリは、単一ファイルの総 byte 数に比例して増大しない。
   長大行、多数一致、多数ファイルでも無制限に蓄積しない。設計で buffer と結果の
   上限、同時実行分を含む予算を明記し、固定した条件で検証可能にする。
7. **既存保護の維持**: 新しい I/O 経路にも workspace boundary と symlink escape
   防止、built-in/custom deny を適用する。拒否対象を新たな範囲読み取りで迂回
   できないこと。bulk-reader は read-only を維持し、共有ツール変更により writer
   の write-scope を緩めない。DESIGN-034r05iHrZUE6WxkSJi9GF の出力・excerpt 制約を維持する。
8. **token count と親 hook**: `count_tokens` は既定 16MB 超を
   `FileTooLargeError` に基づく tool error として扱い続ける。read/grep がその
   cap を超える入力を扱えても token count cap は変えない。親の 100MB 以上の
   直接 Read を block して委譲する既存契約は維持する。

## Acceptance Criteria

- [ ] 16MB 超および 100MB 以上の text fixture に対する範囲 read が、定義した
      cap 以下の内容を返すか規定の cap error となる。全文 `readFile` が呼ばれない
      spy/fake と低層 read の最大要求量検証、または同等のメモリ計測で、返却後の
      truncation だけではないことを示す。
- [ ] ファイル先頭・中間・末尾、空ファイル、改行なしの長大行、UTF-8 境界を
      読むテストが通り、cap 到達と EOF を区別できる。継続可能な場合は後続範囲を
      取得でき、非対応範囲は明示的な error になる。
- [ ] 1MB 超と 100MB 以上の許可された text fixture の後半に置いた既知の一致を
      grep が返す。処理境界をまたぐ一致のテストがあり、対応対象では一致を返し、
      非対応の検索条件は明示的に拒否する。
- [ ] find が 1MB 超・100MB 以上のファイルと binary file を列挙し、内容全体を
      読まないことを spy/fake 等で確認できる。
- [ ] binary の grep 除外、読み取り不能、timeout、結果上限をそれぞれ注入した
      テストで、未処理理由または不完全性が現れ、「一致なし」と区別できる。
      skip 報告の上限超過も識別できる。
- [ ] 同じ設定で 16MB 超から 100MB 以上へ入力を増やし、長大行・多数一致を含めて
      設計で定めたメモリ予算内に収まる。計測時の基準メモリと許容差、または
      buffer/蓄積上限を検証する instrumentation を明記し、全文保持の回帰を検出する。
- [ ] workspace 外、symlink escape、built-in/custom deny の直接 read が拒否され、
      grep/find の結果と skip 報告に deny 対象が出ない。read-only と既存
      write-scope の回帰テストが通る。
- [ ] `count_tokens` は 16MB 超で既存 tool error を返し、cap 未満の canonical
      count は従来と一致する。親の 100MB 以上の直接 Read に対し hook が
      exit 2 と bulk-reader 委譲 guidance を返す既存検証が維持される。
- [ ] 既存 126 テストの対象・期待動作を維持して全件が通り、上記の追加テストが
      通る。実 provider/hook の検証は TASK-034r05iHQeQpDqgk8XoZ8E/0087 の既存 fixture と同様に
      一時ディレクトリへ `apm install` した隔離環境で行い、このリポジトリ内の
      cheaper CLI・hook や cheaper-* スキルによる委譲を実行しない。

## Sources and Relations

- DISC-034r05iHy2gdnpeCjN4yfM: 巨大ファイルに対する現行 I/O の調査と、本仕様の起点。
- SPEC-034r05iHK8bA7gjwZTji8t / ADR-034r05iHrZUE6W82VsDbPq: 親 Read の fail-closed と token count byte cap。
  本仕様はそこで対象外とされた委譲先 I/O を扱い、既存 cap は変更しない。
- SPEC-034r05iHK8bA7ggPjqIA9E: privacy deny の存在秘匿と write-scope の必須制約。
- DESIGN-034r05iHrZUE6WsgJOy6Ny: workspace boundary と sandboxed tool の既存契約。
- DESIGN-034r05iHrZUE6WxkSJi9GF: bulk-reader の read-only、範囲読み取り、出力・excerpt 契約。
- TASK-034r05iHQeQpDqgk8XoZ8E/0087: インストール済み隔離環境を使う配布・hook 検証の既存手順。

cap の数値、範囲指定の詳細、cap 到達応答、検索方式・互換性、メモリ予算は
後続設計で確定する。本仕様では特定の実装方式や依存ライブラリを選定しない。

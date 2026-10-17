---
id: "DISC-034DxZdeAdWoFG3UuTyh5c"
type: "discovery"
status: "resolved"
title: "Post-migration structural inventory and cleanup policy"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source:
    - "Cargo.toml"
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
    - "../adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "../designs/overview.md"
    - "../designs/rust-rig-runtime-migration-architecture.md"
    - "../specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "../adr/consolidate-bundled-provider-implementations-into-a-single-cheaper-provider-crate.md"
    - "../designs/post-migration-structural-cleanup-reorganization.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/post-migration-structural-cleanup-scope.md"
    - "../adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Post-migration structural inventory and cleanup policy

## Exploration Goal

Rust + Rig 移行(DESIGN-034Bbq1qY8WFBdL26tUdBw)完了後の workspace(7 crate)を
棚卸しし、構造整理の「対象・判断基準・優先順位」を確定するための探索。

deep-dive で確定済みの決定軸:

- 主対象は**構造整理**(性能・バイナリサイズ最適化は後続フェーズ)
- 棚卸し+優先度付き方針ドキュメントを先行させ、合意後に実施する
- crate 分割・統合も評価対象に含める(採否は基準で個別判断)
- 本文書の範囲は構造整理のみ。ロギング/プロファイリング環境は別文書で後続

なぜ今か: 移植直後は TS 構造の持ち込み歪み(shim・責務の偏り・test seam の
公開面)が最も見えやすい時期。機能追加が再開する前に整理基準を固定する。

## Context and Constraints

- 外部 contract は不変: envelope `schemaVersion:1` byte parity、exit code
  表、usage.jsonl、hook 4 target、config 書式、配布 contract(cargo
  install + thin launcher)。
- 内部不変条件も維持: Rig 型は `cheaper-rig-adapter` と
  `cheaper-provider-devin` のみ。tokio 境界(core は同期純ロジック)。
  usage 集計の正は hook accumulator。`panic = "unwind"`。
- 実施は behavior-preserving。`mise run check`(biome→fmt→clippy→
  version/doc-id→cargo test→node test→lockfile/payload check)green 維持。
- 測定値は `wc -l` ベース(コメント・空行含む)。可視化スナップショット
  `tmp/architecture.html` は ephemeral な生成物であり既に削除済み
  (gitignore 対象のため再取得不可)。本文の測定表が正とする。

## Inventory Findings

### 健全と確認できた点(confirmed)

- **依存方向は設計通り一方向**。実測 dep グラフは
  `cli → {workflow, provider-devin} → runtime → {rig-adapter, tools} → core`
  で循環なし。composition root(cli による devin plugin 登録)も設計通り。
- **Rig 隔離が守られている**: `rig` 依存は adapter + devin の 2 crate のみ。
- **TODO/FIXME/XXX マーカー 0 件**。移行仕掛かりの未完コードは残っていない。
- **cargo feature なし**: 条件コンパイルの複雑さを持ち込んでいない。
- **JS 互換層の大部分は core に集約済み**: `core::js`(String coercion・
  code_unit_compare)+ `core::json`(canonical number・stringify)を
  workflow/cli も利用する方向になっている。

### 規模と集中

| crate | src LOC | test LOC | src 内構成 |
| --- | ---: | ---: | --- |
| cheaper-tools | 28,211 | 2,834 | root 18,586 / tools/ 9,525 / bin/ 100 |
| cheaper-runtime | 12,259 | 4,505 | auth/ 4,923 / catalog/ 3,055(+39 provider JSON) / root 4,281 |
| cheaper-core | 10,273 | 1,943 | policy 2,335 / worker 2,024 / model 1,822 / 他 |
| cheaper-cli | 7,564 | 5,941 | root 6,024 / hook/ 1,540 |
| cheaper-provider-devin | 5,449 | — | src/tests/ 2,043 を内包 |
| cheaper-workflow | 4,477 | 5,929 | flat |
| cheaper-rig-adapter | 1,997 | 744 | flat |

`cheaper-tools` は workspace src LOC の **40%**。その内訳は glob machinery
(`glob.rs` 2,413 + 生成 `glob_tables.rs` 5,318 ≈ 7.7k)、tool 実装 8 種
(9.5k)、runtime spine(operation 1,767 / resources 1,438 / registry 1,142 /
scheduler 1,113 / access 1,057 ≈ 6.5k)の 3 集団にほぼ分かれる。

### 構造的負債候補

- **F1: cheaper-tools の集中**(confirmed)。40% 集中は単一 crate として大きい。
  ただし内部は root/tools/bin に整理済みで、「大きい」こと自体は違反ではない。
- **F2: dev/test seam が production の公開 API**(confirmed)。
  `cheaper-runtime` の lib.rs が `fake_provider`・`fixture` を pub re-export
  (≈1,854 行が src 内在籍)。`cheaper-cli/src/fake_stream.rs` も同系。
  ただし `CHEAPER_TEST_FAKE_STREAM` は benchmarks が release binary に
  要求するため、単純な `#[cfg(test)]` 移動は不可 — 配置は設計判断要。
- **F3: 移行残滓の re-export shim**(confirmed)。
  `cheaper-rig-adapter/src/js_json.rs` は「Phase 4 で core::json::stringify
  へ移設済み、既存参照維持のための shim」と明記。workspace は全て内部 crate
  (publish しない)なので削除は機械的に可能。
- **F4: JS 互換知識の残る分散**(hypothesis)。`tools::js16`(UTF-16 単位の
  計測・切断)は「JS 方言再現」という同じ知識領域だが tools 内に在る。
  参照が tools 内に閉じるなら現配置で正当化可能。要 caller 調査。
- **F5: `poc-rig` spike の残存**(confirmed)。workspace exclude 済み・
  1,022 行。Phase 0 の実測知見は設計書/ADR に移っているか要確認。
- **F6: `cheaper-runtime` の auth/catalog 偏重**(confirmed)。auth 4,923 +
  catalog 3,055 が orchestration 本体(root 4,281)の 2 倍。oauth×2
  (openai_codex 1,726 + github_copilot 674)は provider 追加のたび育つ
  部分木で、catalog は data JSON 39 件を同梱する self-contained な境界。
- **F7: `cheaper-provider-devin` の `src/tests/` 内包**(confirmed, minor)。
  unit test を src 内に置く配置は他 crate(`tests/` 分離)と非対称。
- **F8: provider 実装の配置規約**(hypothesis — ユーザーレビューで指摘)。
  `cheaper-provider-devin` が provider 専用の単独 crate として workspace
  直下に並んでいる形は、今後の provider 追加を考えると命名・配置規約が
  未確立(`cheaper-provider-<name>` の flat 増殖 vs provider namespace
  への集約)。plugin contract(`CheaperProviderPlugin`)は runtime 所有で
  既に分離済みのため、実装側の置き場だけの問題。

## Evaluation Criteria(整理対象の採否基準)

採否は以下の基準で個別判断する:

1. **凝集度**: 1 module/file が複数の独立責務を持つなら分割対象。
2. **結合度・変更頻度**: 1 変更が常に複数ファイルへ波及する構造は境界誤りの
   兆候。git 履歴の共変で裏付けを取る。
3. **表現ドリフト**: 同一概念(JS 互換・canonical 型)の複数表現は一箇所へ集約。
   変換が必要でも変換処理を一箇所に。
4. **移行残滓**: TS port 由来の shim・dead code・過剰な pub 面は除去。
5. **規模は欠陥シグナルにしない**: 行数・ファイル数だけでは整理対象に
   しない。適切な境界の内側にある大きさは許容する(レビュー確定方針)。
   規模が問われるのは基準 1-3 への違反を伴うときだけ。
6. **不変条件・contract への接近度**: envelope byte parity・hook 集計の正・
   golden との差分検証に近い箇所ほど、動かすリスクに見合う構造的便益を要求。
7. **crate 境界変更の採否基準**(F8 等の再配置・将来の分割候補に適用):
   (a)境界が既存 pub API で自然に表現できるか、(b)現行の一方向依存を
   保てるか、(c)公開面が広がらないか(内部型の新規 public 化を要求
   しないか)、(d)テスト/fixture 配置が自然に移るか、(e)依存分離や
   ビルド並列性の実益が見込めるか。

## Alternatives and Comparison

### F1: cheaper-tools の構造

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| T-A: 現状維持 | 変更ゼロ。内部 module 分割は済 | 40% 集中は今後も拡大方向 | no |
| **T-B: crate 内 module 再編(glob/・runtime spine の subdir 化)** | 小差分で可読性向上。API 不変 | ファイル移動だけで実益が薄い可能性 | **yes(採用)** |
| T-C: crate 分割(例: `cheaper-tools` → spine/impl/glob) | 並列ビルド・責務境界の強制 | 新規 pub 公開面・fixture 移行コスト・依存再配線 | no |

### F2: dev/test seam の配置

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| S-A: 現状(src + pub export)維持 | benchmarks/e2e/regression がそのまま動く | 公開 API に test 実装が残る | no |
| **S-B: `dev-seam` feature gate** | consumer install から seam を除外可能 | mise task・bench・e2e の全経路に feature 有効化が必要。新規 feature の複雑さ導入 | **yes(採用)** |
| S-C: testsupport crate 分離 | 本番 crate の責務が純化 | fake session は runtime 内部 trait を実装するため注入点は pub に残る。移動の実益は限定的 | no |

### F3/F4/F7: 移行残滓と表現ドリフト

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| M-A: js_json shim 削除のみ(最小) | 機械的・即実施可 | js16 配置問題は残る | no |
| **M-B: shim 削除 + js16 の core 移設 + devin tests 分離** | JS 互換知識を core に一元化・配置対称化 | js16 が tools 内に閉じるなら移設は根拠が弱い(caller 調査で確認) | **yes(採用)** |

### F5: poc-rig

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **P-A: 削除** | spike 完了で役目終了・clone 時のノイズ解消 | 実測知見へ戻る参照を失う(知見は設計書/ADR に移管済みか要確認) | **yes(採用)** |
| P-B: 残置 | Phase 0 の参照実装として残る | 保守対象外コードの常駐 | no |

### F6: cheaper-runtime の auth/catalog

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| R-A: 現状維持 | runtime = 「model/auth 周りの services 層」として現在も一貫 | oauth provider 追加のたび肥大 | no |
| **R-B: module 整理のみ** | 低リスク | 構造的緊張は解消しない | **yes(採用)** |
| R-C: `cheaper-catalog`/`cheaper-auth` 分離 | provider 拡張面の境界を強制 | plugin contract・fake seam(3 層に跨る設計)と密結合で切り出しコスト高 | no |

### F8: provider 実装の配置規約

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| PV-A: 現状(`cheaper-provider-<name>` flat 増殖) | provider ごとの依存が完全分離。追加時の規約迷いなし | workspace 直下に provider crate が増えて発見性が下がる。規約が「暗黙」 | no |
| **PV-B: 単一 `cheaper-provider` crate へ devin を module として移す** | 「provider 実装の置き場」が 1 箇所に集約。現行 1 provider の YAGNI に合致 | 将来 provider が異質な依存を持つと 1 crate に deps が合流する → その時点で PV-C へ移行するトリガーを明記 | **yes(採用)** |
| PV-C: `crates/provider/` 配下に provider 別 crate(workspace members に `crates/provider/*` 追加) | 依存分離 + 配置規約の両立 | provider が 1 しかない現状で入れ子 workspace 構成は過剰(YAGNI 違反気味) | no |

## Tentative Conclusions and Hypotheses

- **H1(confirmed)**: 依存方向・Rig 隔離・tokio 境界・composition root は
  設計通り健全。大規模な再配線は不要。整理は「個所の再編」が主対象。
- **H2(hypothesis)**: 構造的負債の主な現れは (a) tools 集中、(b) dev seam
  の公開 API 化、(c) 移行残滓(shim・poc-rig)、(d) JS 互換層の js16 配置、
  (e) provider 配置規約の未確立、の 5 系統。
- **H3(レビュー確定)**: crate **分割**は採用しない方向に収束(tools T-C・
  runtime R-C とも不採用)。crate 境界の再検討が残るのは F8(provider 配置:
  再配置であり分割ではない)のみ。
- **H4(hypothesis)**: 実施順序はリスク昇順で「移行残滓除去(F3/F5/F7:
  shim 削除・poc-rig 削除・devin tests 分離)→ 表現ドリフト集約(F4:
  js16 → core)→ 境界再編(F1: tools module 再編、F6: runtime module 整理)
  → provider 再配置(F8)→ dev seam feature-gate 化(F2)」が妥当。
  feature-gate 化は build/test 経路への影響が最大のため最後に置く。

## Open Questions

- `js16` の caller が tools 内に閉じるか(閉じるなら F4 の移設根拠が弱まる) — M-B 実施前の caller 調査で確定。
- dev seam の feature-gate 化で benchmarks・regression・ollama e2e の全実行経路が維持できるか(mise task への feature 組み込み方式) — F2 実施時に検証。
- `poc-rig` の設計知見が既に design/ADR に移っているか — 削除前に確認。
- ~~F8 の最終形~~ → 確定: **PV-B 採用**。crate 名は `cheaper-provider`
  (devin 実装を `devin` module として内包)。PV-C への移行トリガーは
  「cheaper の依存モデルと合わない異質依存を持つ provider を bundle
  する必要が生じたとき」と明文化。
- 整理実施の単位(task-doc 化するか、1 整理 = 1 commit 粒度か) — spec 化時に確定。

## Promotion Candidates

- [x] spec-doc needed: 構造整理の実施スコープ — 採用方針(F1=T-B, F2=S-B,
  F3/F4/F7=M-B, F5=P-A, F6=R-B, F8=PV-B)の対象一覧・除外・実施順序・
  acceptance criteria(`mise run check` green 維持・envelope parity・
  golden 差分なし・behavior-preserving)。
- [x] adr-doc needed: (1) dev seam の `dev-seam` feature-gate 化(build/test
  経路への影響を持つ設計変更)、(2) provider 実装の配置規約(PV-B vs
  PV-C + crate 命名)。module 再編・shim 削除・poc-rig 削除は spec の実施
  詳細に含めてよい粒度。
- [ ] design-doc: 整理方針の実施設計が必要な粒度になった場合のみ。

## Sources and Relations

- 測定: `Cargo.toml` workspace、`crates/*/Cargo.toml` 依存宣言、
  `find crates -name '*.rs' | wc -l` による LOC 集計(2026-10-10 実施)。
- 可視化スナップショット: `tmp/architecture.html`(crate 依存グラフ・
  モジュール LOC 内訳・run パイプライン・ホットスポット一覧)は
  gitignore 対象の ephemeral 生成物として作成したが既に削除済み。
  内容は本文の測定表に集約されている。
- 上流: DESIGN-034Bbq1qY8WFBdL26tUdBw(crate 責務・境界ルールの正)、
  SPEC(migration 全体)、ADR-034BbhP6hJV7DFtv7k7AGs(Rust+Rig 移行決定)。
- 決定軸の確定経緯: 本セッションの deep-dive(構造整理優先・crate 分割は
  評価対象・方針文書は構造整理のみを範囲)。

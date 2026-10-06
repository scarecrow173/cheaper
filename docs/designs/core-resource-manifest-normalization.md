---
id: "DESIGN-034zJ3dYi8Rs6I6PIyc3Vu"
type: "design"
status: "approved"
title: "Core resource manifest normalization design"
created: "2026-10-04"
updated: "2026-10-04"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/core-resource-manifest-normalization.md"
    - "docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/core-resource-manifest-normalization.md"
  references: []
  defers: []
  deferred-by: []
---

# Core resource manifest normalization design

## Context

coordinator の msg_51dda51d3eb5 への APPROVED: spec (amended) / APPROVED: design を受けた設計。共有 ADR D4/D5 と承認済み spec に従い、core の固定 ResourceClaim/Manifest は変更せず、宣言入力を処理する単一境界を設ける。

## Scope

resources/** と tests、core barrel の resources export のみ。lease、compiler、policy、workspace paths、runtime-pi は対象外。

## Components and Boundaries

- normalization.ts: ScopeDeclaration {path, scope} と ResourceManifestDeclaration {reads?,writes?,produces?}（D5 の mode 無し scope 配列）、ResourceManifestInput（core API 用の ScopeDeclaration | ResourceClaim 配列）を公開する。固定出力型は types.ts。
- normalizeResourceClaim(root, declaration, mode, symlinkPolicy): reads/writes 用 safety scope を正規化する。declaration は workspace-relative path / explicit scope。mode は独立引数とする。
- normalizeResourceManifest(root, input, symlinkPolicy): ScopeDeclaration 入力では bucket から mode を導出し、既存 ResourceClaim 入力では bucket との mode 不整合を拒否する。produces ownership を検査する authoritative manifest 境界。produces の入力は generic claim へ先に近似せず、この関数に渡す。
- predicates.ts: resourceKey（path/scope key）、resourceContains、resourcesOverlap、claimContains、claimsConflict、manifestsConflict を公開する。canonical 入力のみを扱う IO-free predicate。
- resources/index.ts を core barrel の resources export 一行から公開する。

## Data and Control Flow

1. D5 の mode 無し ScopeDeclaration を受理し、manifest bucket から shared/exclusive を一元的に導出する。core API の canonical ResourceClaim 入力は mode / kind / absolute path / scope を検証する。symlinkPolicy / object / mode / scope / path / array を検証する。不正入力は CheaperError(INVALID_REQUEST)。declaration の絶対 path・drive-relative path、NUL、空 path を拒否する。canonical claim は絶対 identity を受理し同じ workspace boundary で real 解決する。
2. canonicalizeWorkspaceRoot を一度実行し、resolveWorkspacePath(root, literal ancestor, symlinkPolicy).real を使用する。正規化処理に独自 filesystem traversal は持たない。
3. literal 宣言は scope を保持し、存在の有無から exact/subtree を推測しない。比較 key のみ Windows 小文字化。POSIX は厳密。
4. raw declaration の reads/writes glob は最初の meta component（* ? [ ] { } ( )、POSIX では backslash も含む）より前の literal ancestor を boundary 検査する。deny では ancestor subtree、within-workspace では root subtree へ広げる。Windows の backslash は separator として component 分割する。leading ! の negation は root subtree。glob tail の .. は拒否し、外への traversal を近似で消さない。
5. raw declaration の produces は meta 無し literal、または最後だけが ** かつ明示 scope=subtree を受理する。dir/*.json、dir/**/file、scope=exact の dir/** は拒否する。filesystem enumeration は行わない。
6. reads/writes 省略/null または resources オブジェクト省略/{} は canonical root subtree の fallback。各 [] は access なし。produces 省略は []、null は拒否。unknown から ownership を作らない。
7. bucket ごとに path比較key / scope / mode / 実path の ordinal sort → key 重複を除去する。Windows alias 表記の代表は同 key で最小実pathとして入力順に依存しない。subtree に包まれる exact を勝手に消さず宣言の粒度を保持する。

## Predicate Contracts

resourceContains は範囲だけを検査する。exact は同じ exact のみを覆い、subtree は本体と全 descendant の exact/subtree を覆う。separator 境界で src/a と src/ab を分離する。claimContains はこれに mode coverage（exclusive は shared/exclusive、shared は shared のみ）を追加する。resourcesOverlap はどちらかの containment。claimsConflict は overlap かつ少なくとも一方 exclusive。manifestsConflict は全 bucket の claim pair に同じ規則を適用する。

## Non-Functional Constraints

依存追加なし。real 解決は既存 boundary に限定する。predicates は filesystem を読まない。

## Risks and Trade-offs

glob 近似は安全側に serialization 範囲を広げる。produces の所有を broad fallback で捏造しない。hard-link alias / 外部 symlink 差替えは既存 boundary の限界を継承する。
within-workspace policy で claim した literal exact/subtree 内の within-workspace symlink 経由の到達先は canonical identity で覆えない場合がある。produces ownership も同じ限界を持ち、ownership は拡大しない。hard-link alias と同様、claim 単位の到達集合保証は行わない。literal exact/subtree は root に拡大しない。

## Deferred Design Concerns

#65-B の compiler inference と #61 の lease は後続 task。
共有 ADR owner への within-workspace literal subtree / produces の到達集合保証 exclusion 追加を coordinator に引き継ぐ（共有 ADR は本 task で変更しない）。filesystem root の既存 paths.ts 配下判定問題も scope 外として別 issue に引き継ぐ。

## Verification Notes

AC1–5 の新規 core tests を先に RED で実行し、implementation 後に同じ test と mise run check を実行する。platform 条件付きケースは host で走った条件と skip を区別して記録する。

## Symlink Safety Rule

本規則の authority は spec の Requirements（glob / platform escape の項）であり、本節はその設計上の根拠を記録する。
within-workspace policy では dir/link.json → other/target.json のように glob に一致する symlink が literal ancestor の外へ（workspace 内で）解決できる。reads/writes の glob は literal prefix の boundary 検査後に workspace root subtree へ広げる。deny policy の glob は nearest literal ancestor subtree のまま、produces は ownership を広げない。AC4a/4b で policy 別 widening と platform 別 escape 解析を検証する。

## Canonical Manifest Admission

mode 不整合の検査は固定 ResourceManifest の実際の array/claim でも行う。canonical claim の path は literal identity であり glob として再解釈しない。core API の canonical 入力は D5 JSON protocol を拡張するものではない。normalization 結果を再度入力でき、安定した dedup を確認する。安全用 reads/writes から produces を自動生成しない。
resources: null、未知の manifest key、同一 bucket 内の宣言と canonical claim の混在、未知の field を拒否する。WorkspaceEscapeError は既存 boundary のまま伝播し、契約入力エラーのみ CheaperError(INVALID_REQUEST) にする。

---
id: "PLAN-034zTJptHTdovrvYW7wQ1k"
type: "plan"
status: "in-progress"
title: "Implement one-dispatch release workflow"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/73"]
  implements: ["../specs/one-dispatch-release-workflow.md"]
  implemented-by:
    - "../tasks/prepare-one-dispatch-release-artifacts.md"
    - "../tasks/wire-one-dispatch-release-publication.md"
    - "../tasks/document-and-verify-one-dispatch-release-migration.md"
  depends-on: ["../adr/one-dispatch-release-workflow.md"]
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/one-dispatch-release-workflow.md"
    - "../adr/one-dispatch-release-workflow.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/release-manifest-and-snapshot-integrity.md"
    - "../test-specs/release-publication-and-native-smoke.md"
  references:
    - "../designs/version-pinned-launcher-acquisition.md"
    - "../../packages/cli/test/e2e/launcher-acquisition.test.ts"
  defers: []
  deferred-by: []
---

# One-dispatch release workflow 実装計画

2026-10-04: APPROVED: 73b-plan（msg_bb3c23ed7158）。

## Goal and Scope

承認済み spec / ADR / design の additive release 経路を実装する。
artifact preparation → native bundle smoke / publish DAG → guide / scoped
verification は同じcritical pathとrelease intentを持つので1 planで扱う。
packages/** / dependencies / LFS / check-apm-payload / 旧release.ymlは変更しない。

## File Map

| path | responsibility |
| --- | --- |
| scripts/lib/release-artifacts.mjs | raw mapping / SemVer-tag / manifest / bounded hash / snapshot boundary |
| scripts/generate-release-checksums.mjs | manifest生成と非変更verify CLI |
| scripts/assemble-release-bundle.mjs | plain staged package archive / dry-run CLI |
| scripts/smoke-release.mjs | archive3hash / APM install / Node-free native launcher smoke |
| scripts/test/release-artifacts.test.mjs | strict manifest / staging / dry-run / source非変更 |
| scripts/test/release-workflow.test.mjs | smoke失敗契約 / workflow DAG・permission・publication order |
| .github/workflows/release-acquisition.yml | 追加workflow_dispatchのresolve/build/assemble/smoke/publish |
| .github/workflows/build-executables.yml | optional expected_version inputとearlygateのみ |
| docs/guides/release.md / distribution.md | 新旧併存・snapshot install・recovery |

## Implementation Sequence

### Slice 1: Release artifact preparation

1. task/IRをin-progressにし、artifact境界の失敗testを作る。
2. node --testでmissingmodule/未実装のREDを記録する。
3. stdlib sharedlibとgenerator/assembly CLIを実装する。
4. manifest拒否ケース、hash、stagingcopy、tarroundtrip、dry-run/source非変更をGREENにする。
5. focused検証evidenceをIRに保存し論理単位でcommitする。

manifest の権威は acquisition design §Release Contract と
launcher-acquisition.test.ts。以下の named unit test を slice 1 の step 1–4 で実装する。

- `manifest uses ASCII and LF`: UTF-8 ASCII subset、WindowsでもLF生成。
- `manifest rejects CR and non-ASCII`: CR/CRLF/BOM/ASCII外を拒否。
- `manifest enforces 64KiB limit`: byte長の上限65536。
- `manifest rejects blank lines`: 空行を拒否。
- `manifest enforces hash delimiter and basename`: 64hex / ASCII space2個 / basenameを要求。
- `manifest requires each target exactly once`: raw3各entryの欠落・重複を拒否。
- `manifest rejects duplicate basenames`: 対象外entryを含む全basename重複を拒否。
- `asset names match launcher resolution`: sh/pwshのraw名とmapping一致。
- `release tag and declarations must agree`: tag / cli / APM / version.json drift拒否。
- `snapshot preserves raw bytes and manifest`: same-run bytesとcli/SHA256SUMS同梱。
- `snapshot leaves source payload unchanged`: sourceの旧payload非変更。
- `snapshot gives POSIX binaries mode 0755`: tar memberの実行permissionをpin。
- `dry-run validates without creating output`: 全input検証のみでoutputを作らない。
- `snapshot refuses existing archives`: 上書き拒否。
- `artifact verification rejects pointer and wrong hash`: LFS pointer / 改変拒否。

### Slice 2: One-dispatch native smoke and publication workflow

1. task/IRを開き、smokefailureとworkflow構造のREDを確認する。
2. smoke-release.mjsを実装しAPMlocalinstall/Node-free実行を所有させる。
3. optional expected_version（未指定skip）を既存reusableに追加する。
4. 新workflowを追加しresolve pre-gateで両stdlib suiteを実行する。
5. fixedSHA / tag/version / native3 / strictfalse / oldtrue / permissions /
   existingRelease拒否 / FETCH_HEAD^{commit} / draft-before-publishをGREENにする。
6. IRにevidenceを記録してcommitする。実Releaseはdispatchしない。

slice 2 の step 1–5 は次の named test と pin を持つ。

- `smoke rejects version mismatch and nonzero exit`: native smokeの失敗を成功扱いしない。
- `workflow pins source SHA and rechecks peeled tag`: FETCH_HEAD^{commit} peel-safe比較。
- `workflow waits for three native bundle smokes`: native3 APM install完了前にpublishしない。
- `workflow publishes complete draft before public release`: raw3+SHA256SUMS+tar.gzをdraftへ一括create後public。
- `workflow refuses existing releases and fails API errors`: 既存draft/public拒否、API取得失敗はfailclosed。
- `workflow dispatch concurrency and token permissions are bounded`: workflow_dispatch only / tag毎concurrency /
  cancel-in-progress:false / contents:read default / writeはpublishのみ / GH_TOKEN=GITHUB_TOKEN。
- `tag passes through env and expected version skips when absent`: tagをenvで渡し、optional gate不在skip。
- `new caller disables strict freshness while old caller keeps it`: 新false/旧true維持。
- `workflow runs focused suites before build`: new pre-gateにstdlib両suiteをwire。
- `cross-OS publisher hashes binaries without executing them`: Linux assemble/publishではWindows binaryはhashのみ、Windows native jobで実行。
- POSIX0755はslice 1のnamed snapshot testを再利用し、workflow artifactのtarで保持する。

### Slice 3: Migration guide and final verification

1. task/IRを開きreleaseguideを1dispatch公開+旧LFS併存に更新する。
2. distributionguideにtar.gz展開→localpathinstallを追加する。
3. stdlibfocused両suite、pnpm lint/typecheck/buildを実行する。
4. 可能ならhost SEA build+Node-free/APMsmokeを補助evidenceとして記録する。
   native3はCI-onlyでありlocalhostsmokeを3targetPASSとは主張しない。
5. chain終端status、managedindex（maintenance plan→apply）、IRindex例外を整合する。
6. scopedexit-audit、doc-idcheckを行い、最終reportを指定Tempに保存しcommitする。

guideの必須事項は acquisition互換Releaseは新経路のみ、旧経路はSHA256SUMS非付随で
launcher fallback failclosed、native3/publishはCI-only検証、manual draft recovery。
slice 3では `release guide distinguishes acquisition and legacy paths` と
`distribution guide installs a plain local snapshot` の named guide review をreportに記録する。
可逆なprose変更をsource-text snapshot testでは検証せず、技術的契約は実script/DAG検証で保護する。

## Step → Spec Mapping and Named Test Anchors

| requirement / AC | slice / step | named test / evidence anchor |
| --- | --- | --- |
| R1 / AC3 | 1.3、2.3–5 | release tag and declarations must agree; workflow pins source SHA and rechecks peeled tag |
| R2 / AC1 | 1.1–4 | manifest uses ASCII and LF; manifest rejects CR and non-ASCII; manifest enforces 64KiB limit; manifest rejects blank lines; manifest enforces hash delimiter and basename; manifest requires each target exactly once; manifest rejects duplicate basenames; artifact verification rejects pointer and wrong hash |
| R2 / AC2 | 1.1–4 | asset names match launcher resolution |
| R3 / AC4 | 1.1–4、2.1–5 | snapshot preserves raw bytes and manifest; snapshot leaves source payload unchanged; snapshot gives POSIX binaries mode 0755; dry-run validates without creating output; snapshot refuses existing archives; smoke rejects version mismatch and nonzero exit; workflow waits for three native bundle smokes; cross-OS publisher hashes binaries without executing them |
| R4 / AC3 | 2.1–5 | workflow publishes complete draft before public release; workflow refuses existing releases and fails API errors; workflow dispatch concurrency and token permissions are bounded; tag passes through env and expected version skips when absent |
| R5 / AC5 | 2.3–5、3.1–2 | new caller disables strict freshness while old caller keeps it; release guide distinguishes acquisition and legacy paths |
| R6 / AC7 | 1.1–4、2.3–5、3.2 | snapshot preserves raw bytes and manifest; new caller disables strict freshness while old caller keeps it; distribution guide installs a plain local snapshot |
| AC6 | 2.4–5、3.3–6 | workflow runs focused suites before build; V6 focused-standard-checks（両suite/lint/typecheck/build/doc-idの実exit0 + scoped audit blocker0をIRに記録） |

AC6のbuild/lint/auditは実command結果で判定し、unit testだけでpassを代用しない。

## Verification Matrix

| slice | command / expected evidence |
| --- | --- |
| 1 | mise exec -- node --test scripts/test/release-artifacts.test.mjs → exit0 |
| 2 | mise exec -- node --test scripts/test/release-workflow.test.mjs → exit0 |
| 3 | mise exec -- node --test scripts/test/release-artifacts.test.mjs scripts/test/release-workflow.test.mjs → exit0 |
| 3 | mise exec -- pnpm lint / pnpm typecheck / pnpm build →各exit0 |
| 3 | mise exec -- node scripts/check-doc-ids.mjs → exit0 |
| 3 | doc-status allをchain+sharedADRでscope → blocking0。globalfindingsはreportのみ |

CLI/package sourceは変更しないためcli package全testは実行しない。
canonical check deferred to coordinator merge run。
新suiteは新workflowpre-gateで実行しmise checkへのwiringは#73-Cへdeferする。

## Risks and Mitigations

- Windows CRLF: manifestはASCII Buffer+LFをwriteしCRを検査する。
- malformed / missing / symlink / pointer / hash mismatch: artifact入力境界でfailclosed。
- sourcepackage書換え: source配下output拒否、owneduniquestagingだけcleanup。
- mutabletag: resolveでSHA固定、publishでFETCH_HEAD^{commit}再照合。
- API failure: Release一覧取得成功前にexistence判定せず、draft完了後だけpublic化。
- oldmanual互換: expected_version空ならskip、既存true/job名/targetsを保持。
- native3実行未検証: CI-onlyとして明記、実Release発火なし。

## Rollback and Follow-up

新workflowとnewscriptはrevert可能。旧Release経路は引き続き使えるが
SHA256SUMSを公開しないのでacquisitionfallbackはfailclosed。
失敗draftはoperatorが確認・手動回復し、自動上書き/削除はしない。
#73-Cはgitpayload/check/CI移行を所有。履歴rewriteは不要。

## Self-Review and Implementation Handoff

全R1-R6/AC1-AC7を3sliceへ対応付け、placeholderなし。既存package/runtime契約を保持。
依存する3sliceをこのworkerで順に実行し、追加workerはdispatchしない。
plan承認後にtest-specを作成してverified-byを張り、その後taskdoc/DAGを生成する。
plan承認ゲート前にtask/codeを作らない。


---
id: "DESIGN-034zTjGVOLExU7QfmMVo3c"
type: "design"
status: "superseded"
title: "One-dispatch release workflow"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  implements: []
  implemented-by: []
  depends-on: ["./version-pinned-launcher-acquisition.md"]
  blocks: []
  supersedes: []
  superseded-by: ["docs/designs/cargo-source-install-distribution-and-thin-path-launchers.md"]
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/one-dispatch-release-workflow.md"
    - "../specs/one-dispatch-release-workflow.md"
    - "../adr/one-dispatch-release-workflow.md"
  derived-by:
    - "../plans/implement-one-dispatch-release-workflow.md"
    - "../test-specs/release-manifest-and-snapshot-integrity.md"
    - "../test-specs/release-publication-and-native-smoke.md"
  verifies: []
  verified-by: ["../test-specs/release-publication-and-native-smoke.md"]
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "./version-pinned-launcher-acquisition.md"
  defers: []
  deferred-by: []
---

# One-dispatch release workflow 詳細設計

2026-10-04: APPROVED: 73b-spec-adr-design（msg_e6292456fa5b）を受け approved。

## Context and Boundaries

overview.md → 本詳細設計の順に読む。SPEC-034zSQV3zqlEncx2GoLKSR /
ADR-034zThoekpLqCcHhYO4MPe を具体化する。共有 ADR-034zGZpjoQlppwkkOs5gur
D1（package依存）/ D6（CLI surface）は維持する。packages/** / dependencies /
LFS / check-apm-payload / 旧release workflow を変更しない。
73b-adr-intent 承認は intent のみであり、本designの自己承認は行わない。

## Workflow DAG

resolve-release → build-executables（既存 reusable、native3）→ assemble-release
→ smoke-bundle（native3）→ publish-release。assemble以降はresolveにも直接依存し、
そのoutputsのSHA / version / tagだけを用いる。全artifactは現在のworkflow runから
downloadし、別runやlatest artifactを参照しない。

- resolve: tag inputをenv経由でscriptへ渡し、canonical cli package /
  packages/apm/apm.yml / committed cli/version.json を照合。tagはv<SemVer>。
  checkoutのHEADをoutputへ保存。以後checkout/refはこの40hex SHAに固定。
- reusable build: optional expected_version inputを追加し、build前にcanonical
  version一致を検査する。input不在/空文字の場合は新gateをskipし、manual dispatchを維持。
  新callerはstrict_freshness:false、旧callerはtrueを維持。
  既存native --version / models smokeとmatrix job名・raw asset名は保持する。
- assemble: stdlib scriptsでraw3のhash/manifestを生成し、plain staged package
  snapshotをtarで圧縮。Node依存packageのinstall / JS再bundleは不要。
  artifact release-bundleはarchiveとSHA256SUMSだけ。rawは既存sea-* artifact。
- smoke-bundle: windows-latest / ubuntu-latest / ubuntu-24.04-armでarchiveを展開。
  全3埋込binaryのhashとinternal/external manifestの一致を検査し、APM local
  install後のlauncher --version / modelsをNode-free PATHで実行する。
  Windowsもnative runnerで実行する。Linux assemble/publishではWindows binaryを
  実行せずhash検証のみ（native smokeはWindows jobが所有）。
- publish: resolve/build/assemble/smokeのsuccess後のみ実行。tag refをfetchし
  FETCH_HEAD^{commit}によるannotated/lightweight tag両対応のpeel-safe比較で
  固定SHAと一致することを再検査。gh api --paginateによる
  Release一覧取得が成功してからexact tag存在を判定し、既存draft/publicを拒否。
  raw3 / SHA256SUMS / tar.gzを一度にdraft作成へ渡す。upload完了後にdraft=false、
  latest=falseで公開する。API失敗を「存在しない」と解釈しない。

## Artifact Contract

| target | Release raw basename | staged payload |
| --- | --- | --- |
| linux-x64 | cheaper-linux-x64 | .apm/hooks/cli/bin/linux-x64/cheaper |
| linux-arm64 | cheaper-linux-arm64 | .apm/hooks/cli/bin/linux-arm64/cheaper |
| win32-x64 | cheaper-windows-x64.exe | .apm/hooks/cli/bin/win32-x64/cheaper.exe |

archive名はcheaper-apm-v<version>.tar.gz、root directoryはcheaper。
含むものはpackages/apmのapm.yml / .apm primitives / committed JS・workers・policy /
canonical version.json / 同run raw3 / cli/SHA256SUMS。
plain snapshotを展開しlocal path依存でinstallする。apm packは使用しない。
sourceのtarget payload directoryをcopy filterで除外するのでLFS pointerや旧bytesを
stagingに持ち込まず、fresh raw3で組み立てる。POSIX binaryはmode0755を設定する。

## Script Responsibilities

scripts/lib/release-artifacts.mjs は対象3mapping、SemVer/tag gate、manifest validation、
bounded streaming SHA256、snapshot assemblyを所有する。new scriptsはNode stdlib
だけで実装し、tar / apm の既存環境commandはchild_processで呼ぶ。
manifest generatorはSHA256SUMSをASCII bytes + LFでwriteしWindowsでもCRLF化しない。

- generate-release-checksums.mjs --asset-dir <dir> --tag v<version>:
  canonical gate → required raw3をregular-fileとして検証 → manifest生成。
  --verifyでは既存manifestのsyntax/coverage/hashを検証し、書き換えない。
- assemble-release-bundle.mjs --asset-dir <dir> --out-dir <dir> --tag v<version>:
  gate/manifest/hashを検証 → owned unique staging → package copy → raw3配置 →
  cli/SHA256SUMS同梱 → tar.gz生成。--dry-runは全input検証とlayout出力のみ。
  archive既存なら拒否する。finallyは自身のstagingだけcleanup。
- smoke-release.mjs --package-dir <extracted-cheaper> --manifest <SHA256SUMS>
  --version <version>: bundle入力/3hashを検証 → temporary consumerにAPM install →
  deployedlauncherを最小PATHで起動。HOME / USERPROFILE / CHEAPER_CONFIG_DIRは
  isolated temporary consumer配下。stdout --version / models ok とexit0を要求。
  Node-free PATHにrunnerのNode/mise pathは入れない。

hashはfile streamで計算する。manifestは65536byte以下を先に確認し、ASCII/CR禁止、
末尾LF、空行なし、64hex/twoASCIIspace/basenameのみ、basename重複拒否、
required raw3を各exactly1で検証する。generatorはこの3entryだけを出力する。
symlink/non-file/LFS pointer/missing/wrong-hashをinput境界で拒否。
pathsはstdlibで解決し、source package配下をoutputとして指定する場合は拒否する。
source treeを削除する処理は持たない。

## Permissions and Failure Recovery

workflow_dispatchのみ。root/job defaultsはcontents:read、publish jobのみcontents:write。
GH_TOKENはpublish stepでGITHUB_TOKENから設定。入力tagのshell直接展開は避けenvを使う。
tagごとconcurrency、cancel-in-progress:false。既存Releaseの上書き/再利用なし。

smoke/hash/version/API failureはpublishを止める。draft生成後の失敗では未公開draftが
残り得る。operatorがrun/error/draft状態を確認し明示的に手動回復した後に再dispatch。
自動削除、resume upload、Release/tag rewriteは行わない。

## Migration and Guide

release.mdには新primary1dispatch手順と旧LFS round-tripの併存を記載する。
acquisition互換Releaseは新経路で生成する。旧経路はSHA256SUMSを持たず、
launcher fallbackはfail closed。distribution.mdにarchive install pathを追記する。
新経路は公開workflowを1dispatchにするが、git-based APM consumerのLFS依存は#73-C
まで残る。version bump時に現行canonical checkのpayload gateが要求するLFS準備を
省略できるとは書かない。archive install例はextract → local path install。
distribution.mdの既存launcher固定version/manifest契約は維持する。

## Verification Design

scripts/test/release-artifacts.test.mjs をnode --testで実行する。
manifest LF/two-space/ASCII/CR/blank/basename/duplicate/64KiB/coverage、
asset mapping、wrong tag/version、stagedraw一致/manifest同梱/source非変更、
dry-run非変更、既存archive拒否、smokeのwrongversion/nonzero拒否、
workflowdependency/permissions/strictfalse/旧trueをpinする。
tag ref再検査 / FETCH_HEAD^{commit} / draft-upload-before-publishもpinする。
新workflowのresolve pre-gateでfocused stdlib suiteを実行し、mise checkへの
wiringは#73-Cへdeferする（mise.tomlはこのsliceで変更しない）。

native3はCI専用で本workerは実dispatchしない。localfocusedtest、fixturetar
roundtripと可能なhostsmoke、pnpm lint/typecheck/build、scopedexit-auditを保存。
canonical check deferred to coordinator merge run。CI-only項目をlocalPASSに含めない。

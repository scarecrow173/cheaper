---
id: "SPEC-034zSQV3zqlEncx2GoLKSR"
type: "spec"
status: "approved"
title: "One-dispatch release workflow"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/73"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: ["../plans/implement-one-dispatch-release-workflow.md"]
  depends-on:
    - "./version-pinned-launcher-acquisition.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: ["../adr/one-dispatch-release-workflow.md"]
  refines: []
  refined-by: []
  derives-from: ["../discovery/one-dispatch-release-workflow.md"]
  derived-by: ["../designs/one-dispatch-release-workflow.md"]
  verifies: []
  verified-by:
    - "../test-specs/release-manifest-and-snapshot-integrity.md"
    - "../test-specs/release-publication-and-native-smoke.md"
  references:
    - "../adr/version-pinned-launcher-acquisition.md"
    - "../designs/version-pinned-launcher-acquisition.md"
    - "../../packages/cli/test/e2e/launcher-acquisition.test.ts"
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  defers: []
  deferred-by: []
---

# One-dispatch release workflow

## What Should Be Built

一度の手動 dispatch で、固定 tag の3 target SEA build、native smoke、strict SHA256SUMS、APM bundle、Release 公開を完結する追加経路。

## Why Now

#73-A launcher は固定 Release の manifest を要求するが、現行 release は raw asset のみ。LFS commit の runner artifact 往復が公開作業を分割している。

## Users and Value

release 担当者は同一 run の公開物を揃えられる。APM consumer は固定 version の asset を checksum 検証して取得できる。

## Scope

### In Scope

- 新 workflow / manifest・bundle・smoke script / stdlib test / release guide。

### Out of Scope

- git payload 削除、LFS / check-apm-payload / 通常 CI 移行、既存 release job の削除、依存変更、launcher・runtime 変更、local publish、実 Release 発火。

## Requirements

- R1: version tag と canonical package version、配布 version.json / APM version は一致し、全 job は同一 commit を使う。公開前に tag ref を再取得し FETCH_HEAD^{commit} を固定 SHA と照合する。既存 Release を上書きせず latest を推測しない。
- R2: raw asset は cheaper-linux-x64 / cheaper-linux-arm64 / cheaper-windows-x64.exe の3個。manifest は UTF-8 ASCII subset / LF only（CR と Windows CRLF 禁止）/ 64 hex + ASCII space 2個 + basename、空行なし、64KiB 以下、各 target exactly 1、全 basename duplicate 禁止。権威は acquisition design §Release Contract / launcher-acquisition.test.ts。
- R3: 3 native runner の --version / models smoke 後に同じ raw bytes を bundle に含める。bundle の APM install / launcher first run を各 target で検証し、全ゲート完了後に公開する。
- R4: 公開権限は publish job の contents:write のみ、token は GITHUB_TOKEN のみ。tag 再検査後に draft へ全 asset を upload してから公開する。公開途中の失敗を成功扱いしない。
- R5: 旧 release / build-executables 採取経路 / LFS の契約を保持。共有 ADR-034zGZpjoQlppwkkOs5gur D1（依存方向）/ D6（CLI surface）は変更しない。
- R6: 新 reusable caller は strict_freshness:false、旧 caller の true / byte-match gate は保持。tar.gz は plain staged snapshot（展開後 APM local path install 用）、apm pack を使わず cli/SHA256SUMS を同梱する。

## Acceptance Criteria

- [ ] AC1: stdlib unit test が UTF-8 ASCII subset / LF only・CR拒否 / two-space / basename / 空行 / duplicate / 64KiB / raw3各exactly1 coverage / hash mismatch の拒否を pin する。
- [ ] AC2: script の asset mapping が両 launcher の resolution と一致する。
- [ ] AC3: workflow 構造検証で fixed SHA、tag/version gate、native smoke、bundle smoke dependency、publish 権限、既存 Release 拒否、FETCH_HEAD^{commit} tag再検査、draft-upload-before-publish を確認できる。focused stdlib suite は新 workflow の pre-gate でも実行する。
- [ ] AC4: staged bundle が同じ bytes と version を持ち、source checkout を書き換えないことを dry-run / unit test で確認できる。
- [ ] AC5: release.md は1 dispatch と旧 LFS 往復の併存期間、CI-only 検証、失敗時の復旧を記載する。
- [ ] AC6: focused script suite / lint / typecheck / build と chain + shared ADR の scoped exit audit が pass。canonical check は coordinator merge run へ defer し、実3 runner / publish PASS を主張しない。
- [ ] AC7: tar.gz snapshot の cli/SHA256SUMS 同梱と新 caller strict_freshness:false / 旧 caller true を構造・dry-run test で pin する。

## Deferred Design Concerns

- #73-C: 全 target consumer smoke の CI evidence 取得後に git payload / check / CI 移行を扱う。現時点で sibling payload が残るので新公開経路だけでは git-based consumer の LFS 往復は廃止されない。

## Sources and Relations

- #73 triage と coordinator brief、DISC-034zSP8CzaOAsOOhj0ZpcW、既存 acquisition ADR / SEA ADR。承認は coordinator の APPROVED 回答後に記録する。

2026-10-04: APPROVED: 73b-spec-adr-design（msg_e6292456fa5b）。指定された tightening を反映。

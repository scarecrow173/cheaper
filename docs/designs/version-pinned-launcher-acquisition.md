---
id: "DESIGN-034zQkHD65kAeG9xVGSoFX"
type: "design"
status: "superseded"
title: "Version-pinned launcher acquisition"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by:
    - "docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/version-pinned-launcher-acquisition.md"
    - "../adr/version-pinned-launcher-acquisition.md"
    - "../discovery/version-pinned-launcher-acquisition.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  defers: []
  deferred-by: []
---

# Version 固定 launcher acquisition 詳細設計

## Context and Boundaries

読み順は既存 overview → 本文書。承認済み SPEC-034zIO2lm7gJPz6Gy4JIQP / ADR-034zIOoBXZtluQdBMlJQge を具体化する。共有 ADR-034zGZpjoQlppwkkOs5gur の D1（package 依存方向）/ D6（CLI surface）は変更せず、D4 の workspace resource lease と acquisition lock は別責務とする。

root apm.yml は development-only で cheaper hook を deploy しない。変更は配布 sh/pwsh launcher、PowerShell hook entry、新規 e2e、distribution guide と usage skill 説明のみ。LFS payload、workflow、scripts、release guide、生成 CLI、hot file は変更しない。

## Release Contract

| host | cache platform / sibling directory | raw Release asset |
| --- | --- | --- |
| Linux x86_64 / amd64 | linux-x64 | cheaper-linux-x64 |
| Linux aarch64 / arm64 | linux-arm64 | cheaper-linux-arm64 |
| Windows x64（pwsh / MSYS / MINGW / CYGWIN sh） | win32-x64 | cheaper-windows-x64.exe |

acquisition 対象は linux-x64 / linux-arm64 / win32-x64 のみとし、それ以外の host では取得を行わず unsupported として明示失敗する。sibling executable が存在する場合は host の supported 可否に関わらず従来どおり exec する（R2 の sibling 優先を全 platform で維持）。PowerShell は OS と native OS architecture を判定し emulation process architecture を使わない。

sibling executable があれば host の supported 可否に関わらず network/cache/version.json を参照せず従来どおり起動する。両 launcher で LFS pointer を拒否し不正 sibling から fallback しない。

sibling 不在時のみ cli/version.json を読む。単一 version string の JSON object、安全な SemVer（数字3要素 + optional prerelease/build）を検証し余分 field/path traversal/malformed JSON を拒否する。POSIX の whitespace 除去 + anchored validation と PowerShell の JSON parse は同じ入力集合を受理する。tag は `v<version>` 固定、latest は使わない。copilot target の deploy では APM 0.28.0 が `*.json` を除外するため `cli/version.json`（および `cli/package.json`）が存在しない（packages/cli/test/e2e/apm-install-targets.test.ts:73-91）。sibling 不在かつ version.json 不在では version 解決不能として fail closed とし取得を行わない。version 解決源の補完（deploy 可能な sidecar 名/配置への変更等）は #73-C の payload 削除までに別変更で解決する。

production base は `https://github.com/scarecrow173/cheaper/releases/download`。CHEAPER_RELEASE_BASE_URL は明示的 trusted HTTPS mirror または literal localhost/127.0.0.1/[::1] の loopback HTTP test origin。userinfo/query/fragment/他 scheme を拒否する。tag/asset は launcher が固定し、redirect は HTTPS のみ（loopback test は HTTP 許容）。curl は接続15秒/全体120秒、Invoke-WebRequest は全体120秒。proxy は curl の HTTPS_PROXY/HTTP_PROXY/NO_PROXY、PowerShell は OS/.NET proxy または HTTPS_PROXY を利用し credential を診断に出さない。

`<base>/v<version>/SHA256SUMS` と同 tag の raw executable を取得する。manifest は UTF-8 ASCII subset / LF / 各行 `<64 hex><two ASCII spaces><asset basename>`、空行なし、上限64KiB。全行を検証し対象 entry は厳密一致で1件（重複拒否）、hex は大小同一視。#73-B は3 asset を同 manifest に列挙して公開する。manifest 欠落/取得不能/不正は fail closed。checksum は破損/誤配送検査であり署名保証ではない。

## Cache State Machine

root は CHEAPER_CACHE_DIR、未指定なら POSIX `${XDG_CACHE_HOME:-$HOME/.cache}/cheaper`、Windows LOCALAPPDATA/cheaper/cache。override は絶対 path とし、解決した root が deployed hook dir 内なら拒否する。cache は同一 user の private directory。owner 自身の書換えや hard-link alias 耐性は主張しない。

```
<cache>/<version>/<platform>/cheaper[.exe]
<cache>/<version>/<platform>/SHA256SUMS
<cache>/<version>/.<platform>.lock/
<cache>/<version>/.<platform>.staging.<unique>/
```

1. platform → sibling 優先 → version/base/cache 検証。
2. version parent 作成後 exclusive mkdir で lock directory 取得（PowerShell は既存 dir を成功とする API を使わない）。owner PID は診断用。1秒間隔/最大120秒待機。既存 lock 以外は即時に書込不可と通知。
3. lock 内で cache 再確認。既存 key は manifest/hash/version を検査し、不正なら network で暗黙修復せず fail closed（key 除去と pair preseed を案内）。binary-only/manifest-only/directory 以外も拒否。
4. key 不在なら同 parent の unique staging directory に manifest → asset を native downloader で取得。hash は sha256sum または shasum -a 256 / Get-FileHash。POSIX は executable bit を付ける。
5. hash 検証後に staged executable --version を実行し exit 0、stdout の固定 version 厳密一致を確認。probe stdin は null/閉じた pipe、stdout/stderr は hook stream と分離。破損/wrong version は公開しない。
6. lock 保持中に staging 全体を同 filesystem の directory rename で公開（binary と manifest を別々に publish しない）。POSIX mv / Windows Directory.Move。第三者が key を配置していれば上書きせず失敗する。
7. lock 削除後に POSIX exec / PowerShell native invocation。元 stdin/argv/stdout/stderr/CLI exit code を保持する。

POSIX trap（EXIT/HUP/INT/TERM）と PowerShell try/finally は自身が所有する staging/lock だけを削除する。通常 network 中断/hash/version/permission error 後に partial を残さない。強制 kill/OS 強制終了の lock は自動奪取せず期限付き timeout。guide に process 不在確認→lock/staging 手動削除を記載する。

## Hook and Diagnostics

hooks.json の PowerShell entry は子 `powershell -NoProfile -ExecutionPolicy Bypass -File ./cheaper.ps1` を通す。script exit が親 fallback を打ち切らない構成とする。pre_read_code/pre_run_command は LASTEXITCODE を返す。Copilot preToolUse の exit 0 swallow、Cursor の非0時 allow JSON + exit 0 を保持。POSIX entry は現行どおり。

取得診断は `cheaper launcher:` prefix / stderr / 非0（1）。network は mirror/proxy/preseed、hash/version は key 除去と正しい pair、lock timeout は owner 確認と手動復旧を案内する。stdout は CLI のみ、成功取得ログは出さない。

## Verification Design

新規 packages/cli/test/e2e/launcher-acquisition.test.ts は host executable と localhost HTTP Release stub を使う。Linux CI は sh、Windows は pwsh と利用可能な Git Bash sh。利用できない runner/host は明示 skip。fixture temp dir で launcher/version.json を copy し sibling なしを作り repo LFS payload は削除しない。

stub は request count、broken body/socket 切断/delay/404/wrong version を制御。Windows は PE executable と real hook stdin JSON を使い ps1 を exe に見立てない。同時2 process で manifest/asset 各1取得・両成功・stage/lock 残存なし。cache hit は HTTP server を停止して検査を証明。未対応 OS/arch は sh の uname stub と Windows native architecture 判定で検証。書込不可は parent が regular file の cache path で deterministic に確認する。

argv の空白/空文字/記号、stdin JSON + hook deny exit 2、probe が stdin を消費しないことを検証する。hooks.json entry 自体を実行し Copilot/Cursor fallback も確認。既存 deployed-executable e2e は sibling payload 互換性 evidence、mise run check が最終 canonical 検証。

## Migration / Deferred Work

新規 docs/guides/distribution.md に既存 install 更新、固定 version と Release 対応、proxy、pair preseed、lock recovery を記載。offline は key 内に executable + 同 tag SHA256SUMS を配置し POSIX executable bit を付ける。binary のみ/別 version は起動しない。

既存 install は launcher 更新後も sibling 優先。dev root は hook 無し。dev consumer の local SEA build は sibling 配置で使用し unreleased version に fallback を期待しない。#73-B の checksum 公開まで現在の Release fallback は fail closed。#73-C の LFS 削除は fresh consumer smoke 後に別変更であり、それまでに copilot target で version.json/package.json が deploy されない制約（`*.json` 除外）への version 解決源の補完も必要（不在時は取得せず fail closed のまま）。

## Graph Checkpoint / Approval

旧 chain refines により複数 plan が lineageComponent に入り非 plan focus が focus-required になる。resume-73a.md の coordinator 承認で design-doc → 承認 → plan-doc → 承認 → test-spec/task を手動 delegate 順に進める。relation/script は改変せず新 plan focus で router 再開。design は coordinator review-protocol/承認回答を待ち自己承認しない。

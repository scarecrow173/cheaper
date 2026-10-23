---
id: "SPEC-034L6i3XmfCaWRHAiw8TTT"
type: "spec"
status: "proposed"
title: "documentation honesty sweep and breaking-change migration guide — claim ≤ 実強制の文書訂正と破壊的変更告知"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
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
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "./permission-model-responsibility-redesign.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/exec-contract-content-audit-removal.md"
    - "../adr/opt-in-exec-deny-patterns.md"
    - "../adr/unified-exec-grant-model.md"
    - "../adr/route-grant-set-projection.md"
    - "../adr/substitute-exec-route-mode.md"
    - "../adr/trusted-route-flag-issuer-redesign.md"
    - "../tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "../adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "./policy-gated-command-tool.md"
    - "./invocation-scoped-command-grants.md"
    - "./pretooluse-route-resolver-and-workflow-delegation.md"
    - "./managed-process-canonical-tool.md"
    - "./workflow-resource-inference.md"
    - "../designs/policy-gated-command-tool.md"
    - "../designs/workflow-resource-inference.md"
    - "../designs/rust-rig-runtime-migration-architecture.md"
    - "../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "../guides/command.md"
    - "../guides/managed-process.md"
    - "../guides/configuration.md"
    - "../guides/workers.md"
    - "../guides/parent-agent-integration.md"
    - "../guides/usage.md"
  defers: []
  deferred-by: []
---

# documentation honesty sweep and breaking-change migration guide — claim ≤ 実強制の文書訂正と破壊的変更告知

## What Should Be Built

権限モデル再設計(#151)分解の最終 slice(7/7)として、docs の
「boundary/sandbox/protection」表現を実強制範囲へ整合させる
文書正直性 sweep と、#163–#168 の破壊的変更を告知する migration
guide を作る。原則は **claim ≤ 実強制** — 提供しない保護を文書で
誇張しない(親 spec R5・親 ADR の「正直性」driver に由来)。

構成要素は 4 つ:

1. **正直性 sweep(guides)** — `docs/guides/` と配布 skill
   (`packages/apm/.apm/skills/`)の stale 表現を実測 inventory
   (discovery §実測 inventory)に基づき in-place 訂正する。
   `command.md:44`(native magic・shim deny 残留)、
   `command.md:133`(call-time metachar 拒否・error code)、
   `managed-process.md:23`(shell/shim/演算子拒否)、
   `managed-process.md:27`(native PE/ELF/Mach-O 限定・win32 shim)
   が確定訂正対象。整合済み箇所(trust/sandbox 限界節・
   execDenyPatterns 節・provenance trust 記述)は現行を保持し、
   文言の均質化が必要な箇所だけ揃える。
2. **canonical disclaimer の明記** — 「cheaper は OS sandbox では
   ない」・「spawn した子プロセスの fs 副作用は強制不能」を
   migration guide と command/managed-process guide の所定節に
   canonical 文言として配置し、spawn API 層の shell-free と
   ENOEXEC→`/bin/sh` fallback(libc 既定動作・保証対象外)の
   区別を exec 系 guide に記述する。
3. **refine sweep(16 doc)** — 親 design「§Supersede / Refine
   一覧」の ADR 7 件・spec/design 9 件すべてへ、対象 doc 内での
   訂正注記(refine 注記)を反映する。supersede はしない。
   front-matter では `refines`/`refined-by` を張らず、意味 link
   は `related`/`references` の backlink で持つ(規約の詳細は
   ADR-034L6ihibvbjmSnKmSffWH)。
4. **migration guide** — `docs/guides/migration.md` を新設し、
   #163–#168 の破壊的変更・挙動変化を issue 別の「変更内容 →
   利用者影響 → 修復/移行手順」形式で列挙する。`docs/README.md`
   の User guides リストへ index row を追加する。

## Why Now

- #163–#168 が全て merge され、旧内容審査層(executable deny
  list・shell/shim/script-ext/native magic/workspace-resident/
  bare `node`/metachar)が mandatory から除去・opt-in 降格済み。
  告知対象の実装が確定したため最終 slice として着手可能。
- 実測で `command.md`・`managed-process.md` に実装と矛盾する
  stale 記述が残っている(同 file 内で migration 注記と
  自相矛盾する箇所あり)。guide は利用者が最初に読む層であり、
  「拒否される」と書かれた挙動が受理される現状は誤設定の温床。
- 親 spec R5(「canonical tool の fs boundary」と「子プロセスの
  副作用は強制不能」の分離)・R9(破壊的変更の列挙)・issue AC
  が本 slice の完了条件を定めている。

## Users and Value

- **policy/route 作者・operator** — 「deny list が守る」
  「workspace に閉じ込める」という誤読を排し、grant は
  shell 相当の信頼として扱う正しい前提で authoring できる。
- **既存利用者(upgrade する)** — `trusted` key を含む project
  routes.yaml が load 全滅する等の破壊的変更に対し、症状別の
  修復手順を単一の guide で得られる。
- **doc 読者・監査者** — 16 refine 対象 doc の stale 記述が
  訂正注記で補正され、各 doc が実装時点の決定記録としても
  現在の真実としても読める。
- **cheaper 保守者** — claim ≤ 実強制の canonical 文言が ADR で
  固定され、以後の制限追加・文書変更の表現基準になる。

## Scope

### In Scope

- `docs/guides/` の stale 訂正(訂必対象 4 箇所 + 均質化)と
  `docs/guides/migration.md` 新設
- `packages/apm/.apm/skills/` の user-facing 表現の確認・軽微訂正
  (`cheaper-usage/SKILL.md` の「deny list」記述への条件付記等)
- canonical disclaimer の配置(OS sandbox 非該当・子プロセス
  副作用の強制不能・ENOEXEC fallback の区別・deny-list 命名)
- refine 対象 16 doc への訂正注記反映(design §Supersede/Refine
  一覧の「変更内容」列を authoritative input とする)
- `docs/README.md` User guides リストへの index row 追加、
  README index 行の status 整合

### Out of Scope

- コード変更 — sweep で code 側の不整合が見つかった場合は別
  issue 化し、本 spec では文書の正直性を優先する(文書訂正のみ)
- 16 対象 doc 以外の doc 全般への過去記述の総書換え — history
  として正しい記録(impl/ir・plans・tasks・handoff・過去
  discovery)は対象外。design §Supersede/Refine 一覧外の ADR/spec
  で mandatory 内容審査を前提に見える箇所(例:
  `opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker`
  の native magic 言及)は、発見した場合に軽微な注記で補うが
  一覧対象の完遂を優先する
- Option E exec-less route ・ OS-sandbox backed mode — 親 chain
  の open/deferred 事項のまま
- release/version bump — migration guide は「現在の main に
  適用される変更群」の告知であり、version 番号の紐付けは
  release 時の責務(記述は issue 番号起点)

## Requirements

- R1. **canonical disclaimer を配置すること** — 「cheaper は OS
  sandbox ではない」・「canonical file tool の workspace
  boundary(E1/E2)・denyPaths・writeScope は cheaper 所有の
  tool surface で強制されるが、spawn した子プロセスの fs
  副作用は強制されない」という文言を migration guide の
  前提節と、`command.md` の trust/sandbox 限界節・
  `managed-process.md` の caveat 節に配置すること。既存の
  正直な記述と重複・矛盾しない表現に揃えること(文言の
  canonical 形は ADR-034L6ihibvbjmSnKmSffWH が規定)。
- R2. **spawn API 層の shell-free と ENOEXEC fallback の区別を
  記述すること** — 「cheaper が使う spawn API は shell を介さず
  argv を直接渡す(shell metachar は executable の literal argv
  として渡り、cheaper が解釈・拒否しない)」と、「POSIX で
  shebang を持たない +X file の実行が ENOEXEC となった際に
  libc が `/bin/sh` fallback するのは OS/libc 既定動作であり
  cheaper の保証対象外(glibc `execvpe` 系は retry、musl は
  retry しない残留差分)」を `managed-process.md`・
  `command.md` に明記すること。
- R3. **guides の stale 4 箇所を訂正すること** —
  `command.md:44`(native magic/shim deny → spawn viability +
  内容審査非実施の現行形)、`command.md:133`(metachar 拒否
  除去・残存 bounds は input schema の `INVALID_INPUT`)、
  `managed-process.md:23`(宣言 argv の内容審査なし・call
  input が task 名のみなのは不変)、`managed-process.md:27`
  (win32 `.exe`/`.com` は CreateProcess 制約の spawn
  viability・unix は +X file を内容無審査で解決・pnpm 等の
  native+entrypoint argv 助言は保持)。訂正後も guide 内で
  記述が自相矛盾しないこと。
- R4. **deny profile の命名規約を適用すること** — sweep 内の
  全編集で `security.execDenyPatterns` を「deterministic
  executable deny-list」(opt-in・既定 `[]`・executable
  basename stem への anchored case-insensitive 完全一致・3
  適用点)と呼び、「保護」「sandbox」「containment」「防御」
  とは呼ばないこと。列挙を免れる経路(改名・代替 binary・
  plugin/extension 機構)が残る旨の但書は既存の正直な記述
  (command.md:63/:148)を維持・再利用する。
- R5. **refine 対象 16 doc すべてに訂正注記を反映すること** —
  各 doc に (a) stale 箇所の訂正(in-place で現在の真実へ
  書き換えるか)、(b) 「Refine 注記」(issue #169・権限モデル
  再設計による変更点・現行真実・ governing doc への link を
  含む短い注記)のいずれかを設けること。ADR は決定時点の
  記録を保持して注記を追記し、living contract として読まれる
  spec の要件本文は in-place 訂正(route spec R1/R12/R15/R34
  の #166–#168 先例)と guide と同じ扱いを許容する。対象と
  訂正方向は discovery §refine 対象一覧表を正本とし、親
  design 表の「変更内容」列を満たすこと:
  - ADR(7 件):
    `command-grant-trust-boundary-and-managed-process-reuse`
    (mandatory deny/内容審査分を opt-in `execDenyPatterns` +
    内容審査除去へ降格した旨の追記)・
    `worker-definition-runtime-policy-runtime-invariants-capability`
    (invariants の適用範囲は canonical surface + 実行契約に
    限る注記 — 3 層モデルは維持)・
    `workspace-filesystem-boundary-cheaper-sandboxed-tool`
    (boundary 強制は canonical tool に限り子プロセスの fs
    副作用には及ばない注記)・
    `pretooluse-auto-routing-via-route-resolver-and-workflow-delegation`
    (C9 single-grant → route-declared grant-set 投影一般化 +
    `trusted` flag → 宣言層 provenance の注記)・
    `per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass`
    (内容審査除去に伴う表現の軽微訂正)・
    `defer-non-exec-routing-to-permission-model-redesign`
    (deferred sketch の採否 — grant-set・条件付き viability
    弱化・record split・substitute 採用、Option E open の
    実装確定結果を追記)・
    `fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront`
    (「`process` は固定 executor・shell なし・metachar なし」
    を統一 grant + `tool:"process"` alias 形へ再記述 + ENOEXEC
    区別)
  - spec/design(9 件):
    `specs/policy-gated-command-tool`(deny・workspace-resident
    要件を opt-in 前提へ降格・内容審査分の除去)・
    `specs/invocation-scoped-command-grants`(deny 参照を opt-in
    条件付き検査へ)・
    `specs/pretooluse-route-resolver-and-workflow-delegation`
    (#166–#168 更新済み — 残差分の整合確認のみ)・
    `specs/managed-process-canonical-tool`(fixed-argv 契約・
    inputs/outputs・「shell executable/構文拒否」を統一 grant
    schema へ写す)・
    `specs/workflow-resource-inference`(§9 declaration seam を
    統一 grant の resource 宣言へ再配線・undeclared process
    の INVALID_REQUEST を fixed grant 前提で再記述)・
    `designs/policy-gated-command-tool`(deny list 部分のみ
    置換の注記 — pin B1/B2・argv pattern A6・C7 manifest・
    route-context 注入は authoritative 継続で supersede しない)・
    `designs/workflow-resource-inference`(同上の再配線注記)・
    `designs/rust-rig-runtime-migration-architecture`(B5 由来
    「node を PATH 解決せず明示 error」記述の訂正)・
    `designs/pretooluse-route-resolver-and-delegated-workflow-execution`
    (viability 分類の条件付き弱化 + substitute retarget +
    provenance trust の注記)
- R6. **refine の front-matter 規約を守ること** — 16 対象 doc
  および本 chain 新規 doc の front-matter では、cross-chain の
  `refines`/`refined-by` edge を張らない(commit 9d76d40 で
  修正済みの graph pollution 再発防止 — ADR-034J8eZHNPqARi25HLFOL6
  :709-710 の明記規約)。意味 link は対象 doc の `related`/
  `references` への backlink(本 spec/ADR/migration guide への
  参照)で表し、per-doc の refine 意図は本 spec R5 の一覧表と
  対象 doc 内の注記本文で表現する。
- R7. **migration guide を作成すること** — `docs/guides/
  migration.md` を新設し、#163–#168 の破壊的変更・挙動変化を
  列挙する。最低限の収録項(discovery §migration guide 収録項
  表を正本):
  - #163 内容審査 7 項目の除去(従来拒否されていた宣言 exec —
    python/ssh/tar/.sh/workspace-resident binary/bare `node`/
    metachar argv が受理される挙動変化)
  - #164 builtin executable deny list 削除 → opt-in
    `security.execDenyPatterns`(seed 例 — command.md の旧 regex
    verbatim・分類別 seed・shell 名 predicate の別追加注記)
  - #165 統一 grant 形状(`process` 節・`tool:"process"` は
    loader 写像 + alias で存続、名衝突 `InvalidRuntimePolicy`、
    `fixed`/`scope` の CLI spec 拒否、project task =
    `scope:"project"`)
  - #166 route grant-set 投影(`grants` は trusted 層限定)+
    `kind:"route"` record の `{command, executed}` split
    (`executed.argv` 平文の注意)
  - #167 substitute-exec mode(trusted 層限定)+ exit は「実行
    した command」の exit
  - #168 project 層 `trusted` key presence 拒否 — 修復 = key
    削除 1 行 or `<userConfigDir>/` へ vendor(user 層 route で
    shadowing 復活・`trusted:true` は user 層で inert 受理)。
    hook 経路の fail-open で症状が `route resolve` まで不可視
    になる注意も記載
  - 各項目は「変更内容 → 利用者影響 → 修復/移行手順」の構成で、
    R1 の canonical disclaimer を guide 冒頭の前提節として
    共有すること
- R8. **文書のみの変更とし、index 整合を取ること** — コード・
  fixture・test を変更しない。`docs/README.md` User guides
  リストへ `migration.md` の行を追加し、`docs/specs/README.md`・
  `docs/adr/README.md`・`docs/discovery/README.md` の本 chain
  行が front-matter status と整合すること。`node
  scripts/check-doc-ids.mjs` が pass すること。

## Acceptance Criteria

- [ ] 「cheaper は OS sandbox ではない」・「子プロセスの fs
      副作用は強制不能」が migration guide と command/
      managed-process guide の所定節に明記されている — issue AC1
- [ ] 「sandbox/boundary/protection」表現と実強制の整合が
      guides・refine 対象 ADR/spec で取れている(discovery の
      掃き出し inventory が全件処理されている) — issue AC2
- [ ] spawn API 層の shell-free と ENOEXEC→`/bin/sh` fallback
      (OS/libc 既定動作・保証対象外)の区別が exec 系 guide
      に記述されている — issue AC3
- [ ] design §Supersede/Refine 一覧の全 16 対象 doc に訂正
      注記(refine 反映)があり、front-matter 規約(R6)を
      破っていない — issue AC4
- [ ] `docs/guides/migration.md` に破壊的変更一覧(deny list
      削除・内容審査除去・grant 形状・`trusted` key 拒否)と
      移行手順が列挙され、`docs/README.md` に index row
      がある — issue AC5
- [ ] 文書のみの差分であり、`node scripts/check-doc-ids.mjs`
      が pass する(README index 行の status 整合を含む)

## Deferred Design Concerns

- 16 対象一覧外の stale 表現の補正 | 一覧外の ADR/spec で
  mandatory 内容審査を前提に見える箇所が残り得る | 発見次第
  軽微な注記で補うか、継続的な sweep 対象として別途管理 |
  sweep 範囲を広げすぎると本 slice が完遂しない — 一覧対象
  の完遂を優先する
- OS-sandbox backed mode | 真の containment 需要が発生した場合 |
  「提供しない」を正直に文書化しておくことで誤期待を防ぐ |
  親 design の Deferred Design Concerns と同一
- migration guide の継続運用 | 将来の破壊的変更ごとの追記先と
  して `migration.md` を canonical にするか、version 単位の
  handoff 化するか | 次の破壊的変更が発生した時点で判断 |
  現時点では issue 別節構成で十分(YAGNI)

## Sources and Relations

- 起点 issue: https://github.com/scarecrow173/cheaper/issues/169
  (権限モデル再設計 7/7 — body は
  `docs/tasks/file-issue-docs-honesty-and-migration-guide.md`
  TASK-034KvxGAbTTWEsA1RbX7cn に canonical 保存)
- 上流 discovery:
  `../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md`
  (DISC-034L67agpaqLudLiaNI822 — claim vs 実強制の実測
  inventory・掃き出し表現一覧・migration guide 収録項)
- honesty policy・refine 規約・guide 置き場所の決定 ADR:
  `../adr/documentation-honesty-sweep-and-migration-guide.md`
  (ADR-034L6ihibvbjmSnKmSffWH)
- 親 chain: `../designs/permission-model-responsibility-redesign.md`
  (DESIGN-034KuDkWOmNXiBddEQ4XWK — §Supersede/Refine 一覧が
  16 対象の authoritative input)・
  `./permission-model-responsibility-redesign.md`
  (SPEC-034KuqPkTK2b4mHsykH9A5 — R5/R9)・
  `../adr/enforce-declared-contracts-over-content-audit.md`
  (責務モデル)・`../discovery/permission-model-responsibility-redesign.md`
- 破壊的変更の実装側 ADR(告知材料):
  `../adr/exec-contract-content-audit-removal.md`(#163)・
  `../adr/opt-in-exec-deny-patterns.md`(#164)・
  `../adr/unified-exec-grant-model.md`(#165)・
  `../adr/route-grant-set-projection.md`(#166)・
  `../adr/substitute-exec-route-mode.md`(#167)・
  `../adr/trusted-route-flag-issuer-redesign.md`(#168)
- refine 規約の前例: commit `9d76d40`(cross-chain `refines` →
  `related`)・`../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
  :709-710・route spec の #166–#168 in-place 訂正(R1/R12/R15/R34)
- guide 規約: `../guides/release.md`・`../guides/distribution.md`
  (front matter 無し・`docs/README.md` User guides 手動 index)

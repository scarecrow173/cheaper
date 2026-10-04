---
id: "ADR-034A8q6O5VZdEVmlvLrWP2"
type: "adr"
status: "accepted"
title: "managed process の named task 認可と shell-free lifecycle を固定する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/59"]
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
  derives-from: ["../discovery/managed-process-canonical-tool.md"]
  derived-by: []
  verifies: []
  verified-by: []
  references: ["./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Managed process named task lifecycle

## Context and Problem Statement

canonical registry が入力検証と accounting、OperationManager が lifecycle、scheduler が admission を所有する。process のために別 runtime や shell adapter を作ると D1/D5/D7 が崩れる。

## Considered Options

* named task + canonical executor + scheduler pool（採用候補）。
* Pi shell helper 注入（shell-free と Workflow 共有に違反するため却下）。
* process 独立 job kind / 専用 operation manager（責務重複のため却下）。

## Decision Outcome

Chosen option: named task + canonical executor。認可/config は core、spawn/capture/kill は tool-runtime、policy-aware metadata を Workflow が利用し、Pi は無変換 adapter に留める。D1/D2/D4/D5/D7 を変更しない。

policy の optional `process` は `{maxConcurrent:1,tasks:{}}` を既定とする。task は command:string[]、inputs/outputs:string[]、cwd?:string、env?:Record<string,string>。各 task の必須三 field は partial policy でも必要。project の maxConcurrent は trusted cap との min。task 宣言は explicit authorization（project でも定義可能）として既存 merge 方式を使う。capture は固定 65536 bytes（stdout/stderr 合計）とし、新しい restriction key は追加しない。

native executable を shell:false で spawn する。`node` は process.execPath の実体名が node/node.exe の場合だけそれを使用し、SEA の cheaper executable を Node として起動しない。それ以外の `node` や bare command は trusted host PATH の native executable を探索する。Windows .cmd/.bat/.ps1 shim は拒否し、native executable または `node` + package CLI script を明示する。cwd と宣言 I/O は core boundary、argv は shell/メタ構文を core config validation で拒否する。env は task に明示した値のみ（Windows の OS 起動に必要な SystemRoot のみ最小補完）。

POSIX は専用 process group の強制終了、Windows は native taskkill.exe /T /F による tree 終了。cancel reason を保存し、cleanup 完了と child close の観測後に reject する。stdout/stderr 合計 cap 以降も stream を drain し、非ゼロ exit/spawn failure は bounded details を持つ TOOL_EXECUTION_FAILURE。

stdin は ignore。setsid/re-parent で管理下を離れた孫の cleanup と、正常終了後の孫残留検査は保証しない。project task の登録は任意 executable の事前認可となるため、信頼できる project policy のみに process を許可する。task 宣言と .cheaper/** deny は OS の未宣言 access を止める sandbox ではない。

coordinator 承認: msg_126e0d3e23d0、spec-approval / adr-acceptance / design-approval。hot files と index.ts export 変更も承認済み。

readonly 宣言だけ shared、outputs がある task は exclusive。scheduler の optional pool 宣言を process sublimit に使い、capacity は実 settle まで保持する。宣言は信用するもので OS のアクセス sandbox ではない。

## Implementation Plan

* Affected paths: core policy/process、vocabulary、worker mutation predicate、tool-runtime catalog/registry/invocation/scheduler/process、runtime-pi registry/run、workflow resource-inference、builtin policy 二箇所。
* Constraints: hot-file は design gate で coordinator 承認、依存/manifests/root scripts は変更しない。pool は admission の一部として実装し、別 semaphore を Worker ごとに作らない。
* Consequences: Windows shim は対応範囲外。宣言外 access と hard-link/symlink 外部差替えは観測できない。resource lease は #61 に委ね、現行排他は conservatively 保持する。

## Verification

* [ ] 実 process の bounded output、非ゼロ exit、秘密 env 非継承、子孫 cancel と adapter test が通る。
* [ ] policy/schema/load の trusted→project cap test、producer ownership test、scheduler pool/cancel/accounting test が通る。
* [ ] focused suite、pnpm lint/typecheck/build が通り、canonical check は coordinator merge run に defer。

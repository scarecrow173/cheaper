---
id: "DESIGN-034zIUgKgoIuBD5BY6jLjf"
type: "design"
status: "approved"
title: "Canonical Tool Runtime と Pi adapter"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  derives-from: ["../specs/canonical-tool-runtime.md", "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---

# Canonical Tool Runtime と Pi adapter

## System Boundaries

共有 ADR D1/D7 を実装する。Worker Runtime は model/session、Worker timeout/retry、
出力抽出を所有する。Tool Runtime は deterministic handler、catalog、schema 検証、
boundary 付き実行、budget/meter/journal と invocation lifetime を所有する。
Workflow Engine は job DAG/materialization/aggregate result を所有し、この slice では変更しない。

## Components and Data Flow

1. tool-runtime に read/grep/find/count_tokens と walker/line-framer/limits を移動。
2. canonical descriptor は Pi 型を使わず schema と typed handler を持つ。入口で core
   compileSchema を一度実行する（validator は構築時に compile、入力検証は call ごと一回）。
3. catalog は全 7 tool の canonical input/output schema を公開。read はページ情報、
   count_tokens は token count を details に保持。他の result details は helper の JSON を保持。
4. direct registry は許可 tool と注入 handler を解決し、budget 外側/meter 内側を一回適用。
   unknown/未注入 helper は明示拒否。counter は await 前に更新し sibling race を防ぐ。
5. runtime-pi は boundary operations を Pi helper に注入し、その execute を canonical
   handler port に渡す。Pi adapter は canonical schema と execute を ToolDefinition へ露出。
   helper の prepareArguments/prompt metadata と same-file mutation queue semantics を保存する。
6. runtime-pi の旧 factory/utility import は thin adapter または互換 re-export として残す。
   budget/meter の旧公開 wrapper は共通の Pi-free accounting 実装を再利用する。
7. MutationJournal は tool-runtime に移し、旧 journal/index は同じ class を re-export。

## Public API Shape

```ts
interface ToolCallContext { toolCallId?: string; signal?: AbortSignal }
interface ToolRuntime {
  catalog: ToolCatalog;
  journal: MutationJournal;
  budget: ToolBudget;
  meter: ToolMeter;
  execute(name: string, input: unknown, context?: ToolCallContext): Promise<CanonicalToolResult>;
  abort(): void;
  drain(): Promise<void>;
  dispose(): Promise<void>;
}
interface InvocationToolRuntime {
  readonly root: string;
  readonly symlinkPolicy: SymlinkPolicy;
  // createWorkerScope は root/policy を確認して ownerId の独立 lifetime を作る。
  createWorkerScope(options: WorkerScopeOptions): ToolRuntime;
  abort(): void;
  drain(): Promise<void>;
  dispose(): Promise<void>;
}
interface WorkerExecutionBinding {
  runtime: InvocationToolRuntime;
  ownerId: string;
  parentLease?: ResourceLease;
}
```

ToolCatalog の生成には root、model、session、handler の初期化を必要としない。
ResourceLease は未実装であることを型で表し、実際の値を受け取る経路は reject する。
ToolRuntime は scheduler service を表す型ではなく、この slice の canonical execution
入口と実行中 call の lifetime を表す。InvocationToolRuntime も manager の代理 no-op を持たない。

## Ownership and Cleanup (D7)

InvocationToolRuntime facade と WorkerExecutionBinding を公開し、RunWorkerDeps.execution
を optional に追加する。facade が worker scope を作り、scope は registry/journal、
AbortController、実行中 promise を保持する。abort は新規開始を拒否し running signal を伝える。
drain は scope の AbortController を abort した後、実行中 promise の settle を abort grace
（abortGraceMs、既定 5000ms — promptWithAbortGrace と同一の猶予契約）で bound する。grace
経過後は待機を打ち切り、未 settle の promise は tracking から放棄する（late settle は無害に
破棄）。dispose は abort + bounded drain と scope 解放を行い、必ず有限時間で return して
single-Worker の outer finally を永久ブロックしない。grace 経過で drain を打ち切ることを
contract test で pin する。
single-Worker は runWorker が facade 一つを所有して outer finally で dispose する。
borrowed binding は owner scope のみ dispose し facade を dispose しない。
root/symlink policy 不一致、duplicate owner、parentLease の未対応利用は closed。

OperationManager/ToolScheduler/ResourceManager/ResourceLease は未実装の型として明示し、
インスタンスや成功する no-op を作らない。共有 scheduling/concurrency policy の固定は
#57 の service 導入時に実装し、この slice が global scheduling を提供するとは主張しない。

## Safety and Compatibility

- filesystem boundary/deny/writeScope は既存実装を変更せず移すか helper operations に残す。
- CanonicalToolResult の text/details を再解釈しない。error は throw のまま。
- malformed schema 拒否は INVALID_INPUT。helper-backed 未登録は UNKNOWN_TOOL。
- budget で拒否された call と access denied を rejectedToolCalls に一回計上。
- meter は成功した content[].text だけを countTextTokens で集計。
- root/scope abort と budget admission は handler 開始前に確認する。

## Verification

同一実 workspace を direct と Pi adapter で実行し content/details と拒否を比較する。
注入 helper の writeScope/deny を比較し、malformed input が副作用を起こさないことを確認。
既存 huge-file/symlink/grep batchMaxBytes、sibling admission/abort、same-file write/edit、
partial mutation、dependency guard と canonical mise run check を実行する。

## Risks and Deferred Work

helper 依存の schema は canonical descriptor に集約し、pinned Pi helper schema と
contract test で一致を確認する。Pi 自身の外側 argument validation は変更できないが、
canonical runtime はその validation に依存せず独立に一回検証する。
#56/#57/#61 の service 実装、lease borrowing、global concurrency は後続 issue。
core の countFileTokens は signal を受け取らず、開始済み tokenization を途中停止できない。
scope は開始前に abort を拒否し、開始済み call は abortGraceMs で bound された drain まで待つ
（grace 経過で放棄される未完の tokenization は late settle として破棄される）。


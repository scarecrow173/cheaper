//! Port of `packages/tool-runtime/src/registry.ts`(T2-7)。
//!
//! tool dispatch の orchestration 層。definition(schema 検証・単一 tool
//! 実行)の外側で、runtime abort → registry lookup → prepareArguments →
//! input validation → signal 合成 → declaration 導出 → budget admission →
//! scheduler → resource access → active operation binding →
//! token metering → definition.execute の順に責務を重ねる。
//!
//! TS と同じく `observeAccess` は「現在実行中の operation」に紐付けるが、
//! JS の `AsyncLocalStorage` 相当は `tokio::task_local!` で再現する
//! (scheduler→execute callback は同一 task の await chain のため
//! task_local で束縛できる)。

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::command::{resolve_command_policy, ResolvedCommandPolicy};
use cheaper_core::policy::process::{resolve_process_policy, ResolvedProcessPolicy};
use cheaper_core::resources::predicates::claim_contains;
use cheaper_core::resources::types::{ClaimMode, ResourceScope};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::schema::{compile_schema, CompiledSchema, SchemaKind};
use cheaper_core::worker::validate::is_mutating_tool;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde_json::Value;
use tokio::sync::{Notify, OnceCell};
use tracing::Instrument;

use crate::access::{
    AccessObserver, AccessSnapshot, ActiveResourceAccess, ResourceAccess, ResourceScopeOptions,
    ScopeAccessOptions,
};
use crate::accounting::{
    create_tool_budget, create_tool_meter, ToolBudget, ToolBudgetOptions, ToolBudgetRunner,
    ToolMeter, ToolMeterRunner,
};
use crate::artifact::ArtifactPorts;
use crate::catalog::{CanonicalToolResult, ToolCatalog};
use crate::definition::{validate_tool_input, CanonicalExecuteFuture, CanonicalToolDefinition};
use crate::journal::MutationJournal;
use crate::operation::{
    AbortController, AbortListenerId, AbortReason, AbortSignal, CreateOperationOptions,
    OperationManager, OperationManagerOptions, OperationRecord, OperationState,
};
use crate::resources::{
    invalid_resource, LeaseKind, ParentLeaseSource, ResourceLease, ResourceManager, ResourceRequest,
};
use crate::scheduler::{OperationDeclaration, ReadinessFuture, ToolScheduler};
use crate::tools::boundary::{
    create_edit_operations, create_ls_operations, create_write_operations,
    BoundaryOperationsOptions,
};
use crate::tools::command::{
    command_operation_declaration, command_tool_definition, pin_command_grants, CommandToolOptions,
};
use crate::tools::count_tokens::{count_tokens_tool_definition, CountTokensToolOptions};
use crate::tools::edit::edit_tool_definition;
use crate::tools::find::find_tool_definition;
use crate::tools::grep::{grep_tool_definition, GrepToolOptions};
use crate::tools::grep_eval::EvalSpawn;
use crate::tools::ls::ls_tool_definition;
use crate::tools::process::{
    process_operation_declaration, process_tool_definition, OnTimeout, ProcessToolOptions,
};
use crate::tools::read::{read_tool_definition, ReadToolOptions};
use crate::tools::write::write_tool_definition;
use crate::walker::WalkOptions;
use crate::ToolError;

// ---------------------------------------------------------------------------
// AsyncLocalStorage 相当(active operation → observeAccess binding)
// ---------------------------------------------------------------------------

tokio::task_local! {
    /// `active` の AsyncLocalStorage 相当。
    static ACTIVE_OPERATION: ActiveResourceAccess;
}

// ---------------------------------------------------------------------------
// Options / binding
// ---------------------------------------------------------------------------

/// `ToolRuntimeOptions extends ResourceScopeOptions`。
///
/// `handlers`(TS の `InjectedToolHandlers`)は存在しない — Rust では write/
/// edit/ls も含め全 8 tool が native のため injection seam は不要。
pub struct ToolRuntimeOptions {
    // `ResourceScopeOptions` 部分。
    pub resource_manifest: Option<Value>,
    pub write_scope: Option<Vec<String>>,
    pub read_strictness: Option<crate::resources::ReadStrictness>,
    pub artifacts: Option<ArtifactPorts>,
    /// `Partial<ProcessPolicy>` の raw JSON(`resolve_process_policy` 入力)。
    pub process: Option<Value>,
    /// `Partial<CommandPolicy>` の raw JSON(`resolve_command_policy` 入力)。
    /// `names` に `Command` を含む runtime は nonempty-grants gate +
    /// writeScope 拒否 + per-scope pin が registration 境界で効く
    /// (design §(b)/(c)/(g))。
    pub command: Option<Value>,
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
    pub names: Vec<ToolName>,
    /// worker frontmatter `cheaper.optionalTools` 由来の縮退許可集合 —
    /// content-gate(nonempty tasks/grants)不成立時に `names` から
    /// 未登録へ縮退してよい scope-gated tool の opt-in 宣言
    /// (design §(d))。`names` と同じく scope 単位の宣言であり、
    /// policy ではない。非 opt-in の構築 site は空 vec を渡す。
    pub degradable: Vec<ToolName>,
    pub max_tool_calls: u64,
    pub on_limit: Box<dyn Fn() + Send + Sync>,
    /// shared journal(未指定なら runtime 内に新規生成)。
    pub journal: Option<Arc<MutationJournal>>,
    pub denylist: Option<PathDenylist>,
    /// 親 job scope の lease(`ResourceManager` が所有権を検証する)。
    pub parent_lease: Option<ResourceLease>,
    /// owner scope signal(TS `options.signal` — invocation が composite
    /// signal を渡し、standalone runtime は call 側 signal のみ受ける)。
    pub signal: Option<AbortSignal>,
    pub abort_grace_ms: Option<f64>,
    /// `Partial<ConcurrencyPolicy>` の raw JSON。
    pub concurrency: Option<Value>,
    /// Rust 差替 seam: grep eval subprocess の spawn spec(production は
    /// `current_exe` + `__grep-eval`、test は `test_spawn` 相当を注入)。
    /// TS では helper は worker file で固定。
    pub eval_spawn: Option<EvalSpawn>,
}

/// `createWorkerScope` が `createToolRuntime` へ渡す共有基盤
/// (TS `binding`)。manager/scheduler/resources は invocation 内で共有する。
#[derive(Clone)]
pub struct OperationBinding {
    pub manager: OperationManager,
    pub scheduler: ToolScheduler,
    pub owner_id: String,
    pub resources: ResourceManager,
}

/// `context.signal`(`execute` 第 3 引数)。
pub struct ToolCallContext {
    pub signal: Option<AbortSignal>,
}

// ---------------------------------------------------------------------------
// managed process tracking(`managedProcesses` Set 相当)
// ---------------------------------------------------------------------------

/// process tool の未 settle 実行を数える。`drain` は 0 になるまで待つ
/// (TS `managedProcesses: Set<Promise>` + `Promise.allSettled` 相当 —
/// Rust では counter + notify で集合の settlement を表現する)。
#[derive(Default)]
struct ManagedProcesses {
    count: AtomicUsize,
    notify: Notify,
}

struct ManagedGuard<'a> {
    set: &'a ManagedProcesses,
}

impl Drop for ManagedGuard<'_> {
    fn drop(&mut self) {
        self.set.count.fetch_sub(1, Ordering::SeqCst);
        self.set.notify.notify_waiters();
    }
}

impl ManagedProcesses {
    fn track(&self) -> ManagedGuard<'_> {
        self.count.fetch_add(1, Ordering::SeqCst);
        ManagedGuard { set: self }
    }

    /// `Promise.allSettled([...managed])` 相当。settlement 済みの tracked
    /// call(=guard drop)だけを待ち、drop 時の notify で起床する。
    /// `notify_waiters` は register 前の通知を失うため、`Notified` を
    /// `enable()` で先に登録してから count を確認する(tokio 1.53+)—
    /// check-then-register の隙間で通知が落ちる lost wakeup を防ぐ。
    async fn wait_idle(&self) {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            let _ = notified.as_mut().enable();
            if self.count.load(Ordering::SeqCst) == 0 {
                return;
            }
            notified.await;
        }
    }
}

// ---------------------------------------------------------------------------
// registered tool entry / tool construction ctx
// ---------------------------------------------------------------------------

/// `registered` map の value(TS `{definition, schema}` pair)。
struct RegisteredTool {
    definition: CanonicalToolDefinition<'static>,
    schema: CompiledSchema,
}

/// execute closure が毎呼出しで borrowed tool を再構築するための shared
/// context。`ReadToolOptions` 等は borrowed field を持つため `'static`
/// closure の env に直接閉じ込められない — owned ctx を `Arc` 共有し、
/// tool を future 内で構築する(construction は ref struct の初期化のみで
/// 軽量。TS は handler 構築時に 1 度 tool を作る)。
struct ToolCtx {
    root: String,
    symlink_policy: SymlinkPolicy,
    denylist: Option<PathDenylist>,
    journal: Arc<MutationJournal>,
    artifacts: Option<ArtifactPorts>,
    owner_id: String,
    /// write/edit/ls の `BoundaryOperationsOptions.scope`。
    scope: ResourceScopeOptions,
    process: ResolvedProcessPolicy,
    /// declared-shape の resolved command policy(design §(g))。
    command: ResolvedCommandPolicy,
    /// grant name → pin 済み canonical executable path(per-scope pin、
    /// `names` に `Command` を含む場合のみ構築。それ以外は空)。
    command_pins: BTreeMap<String, String>,
    eval_spawn: EvalSpawn,
    on_timeout: Option<OnTimeout>,
    observe_access: Arc<AccessObserver<'static>>,
}

impl ToolCtx {
    fn boundary(&self) -> BoundaryOperationsOptions<'_> {
        BoundaryOperationsOptions {
            observe_access: Some(&*self.observe_access),
            root: &self.root,
            symlink_policy: self.symlink_policy,
            journal: Some(&self.journal),
            denylist: self.denylist.as_ref(),
            scope: self.scope.clone(),
        }
    }

    fn walk(&self) -> WalkOptions<'_> {
        WalkOptions {
            observe_access: Some(&*self.observe_access),
            root: &self.root,
            symlink_policy: self.symlink_policy,
            denylist: self.denylist.as_ref(),
        }
    }

    fn grep(&self) -> GrepToolOptions<'_> {
        GrepToolOptions {
            root: &self.root,
            symlink_policy: self.symlink_policy,
            denylist: self.denylist.as_ref(),
            observe_access: Some(&*self.observe_access),
            eval_per_file_ms: None,
            eval_total_ms: None,
            scan_deadline_ms: None,
            chunk_bytes: None,
            max_line_bytes: None,
            batch_max_lines: None,
            batch_max_bytes: None,
            output_max_bytes: None,
            output_line_max_chars: None,
            skip_footer_max_entries: None,
            skip_path_max_chars: None,
            skip_footer_max_bytes: None,
            eval_spawn: Some(self.eval_spawn.clone()),
            eval_reply_max_bytes: None,
            now: None,
        }
    }

    fn read(&self) -> ReadToolOptions<'_> {
        ReadToolOptions {
            root: &self.root,
            symlink_policy: self.symlink_policy,
            denylist: self.denylist.as_ref(),
            observe_access: Some(&*self.observe_access),
            artifacts: self.artifacts.as_ref(),
            owner_id: Some(self.owner_id.as_str()),
            max_bytes: None,
            chunk_bytes: None,
        }
    }

    fn count_tokens(&self) -> CountTokensToolOptions<'_> {
        CountTokensToolOptions {
            observe_access: Some(&*self.observe_access),
            root: &self.root,
            symlink_policy: self.symlink_policy,
            denylist: self.denylist.as_ref(),
        }
    }

    fn process(&self) -> ProcessToolOptions {
        ProcessToolOptions {
            root: self.root.clone(),
            symlink_policy: self.symlink_policy,
            process: self.process.clone(),
            on_timeout: self.on_timeout.clone(),
            denylist: self.denylist.clone(),
            write_scope: self
                .scope
                .write_scope
                .clone()
                .map(|scope| scope.into_iter().collect()),
        }
    }

    fn command(&self) -> CommandToolOptions {
        CommandToolOptions {
            root: self.root.clone(),
            symlink_policy: self.symlink_policy,
            command: self.command.clone(),
            pins: self.command_pins.clone(),
            on_timeout: self.on_timeout.clone(),
            denylist: self.denylist.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// ToolRuntime
// ---------------------------------------------------------------------------

struct RuntimeInner {
    catalog: ToolCatalog,
    journal: Arc<MutationJournal>,
    /// `Arc` — `observeAccess` closure が同一 instance を共有する。
    access: Arc<ResourceAccess>,
    /// `registered`(key は canonical `ToolName`。lookup は `name as ToolName`
    /// 相当の `ToolName::from_canonical` で行う)。
    registered: HashMap<ToolName, Arc<RegisteredTool>>,
    /// `opts.names` 順(definitions/external defs の order 保持)。
    names: Vec<ToolName>,
    accounting: ToolBudgetRunner,
    metering: ToolMeterRunner,
    controller: AbortController,
    manager: OperationManager,
    scheduler: ToolScheduler,
    resources: ResourceManager,
    owner_id: String,
    abort_grace_ms: f64,
    /// `binding` 由来か(TS `if (!binding) await manager.dispose()` —
    /// invocation 共有分は runtime が dispose しない)。
    owns_managers: bool,
    /// `opts.parentLease`(constructor 引数)。
    options_parent_lease: Option<ResourceLease>,
    /// `let parentLease = opts.parentLease`(ready() の start callback が
    /// 確立する scope lease)。
    parent_lease: Arc<Mutex<Option<ResourceLease>>>,
    external_signal: Option<AbortSignal>,
    external_listener: Option<AbortListenerId>,
    scope_listener: AbortListenerId,
    /// `readyPromise` memoization — TS は promise 自体を memoize する
    /// ため、一度 reject された `ready()` は以降も同じ rejection を
    /// 返す(retry しない)。`Result` 自体を `OnceCell` に格納し
    /// `get_or_init` で共有する(in-flight の concurrent `ready()` も
    /// 同一 request を待つ)。
    ready_cell: OnceCell<Result<ResourceLease, AbortReason>>,
    /// `readyPromise !== undefined` — `ready()` 呼出し済みか(in-flight を
    /// 含む。`OnceCell::initialized` は完了後のみ true のため別途追跡)。
    ready_requested: AtomicBool,
    /// `disposed` promise memoization。
    disposed: OnceCell<()>,
    managed: ManagedProcesses,
    tool_ctx: Arc<ToolCtx>,
    /// worker scope 消滅時に invocation の `scopes` 集合から自分を外す
    /// hook(TS は `scope.dispose` を wrap して remove する)。
    dispose_hook: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

/// `ToolRuntime`(TS `createToolRuntime` が返す object 相当)。
/// `Clone` は `WorkerExecutionBinding` が runtime object を保持するため
/// (shared state は全て `Arc` 裏の同一 instance)。
#[derive(Clone)]
pub struct ToolRuntime {
    inner: Arc<RuntimeInner>,
    /// `definitions`(TS `{...definition, execute: delegate}`。`Weak` 経由で
    /// self-cycle を切る)。
    definitions: Arc<Vec<CanonicalToolDefinition<'static>>>,
}

fn invalid_resource_err(message: &str) -> ToolError {
    Box::new(invalid_resource(message))
}

/// `throw reason`(TS は reason object そのものを throw)。
fn throw_reason(reason: AbortReason) -> ToolError {
    Box::new(reason)
}

fn unknown_tool(name: &str) -> ToolError {
    Box::new(
        CheaperError::new(
            ErrorCode::UnknownTool,
            format!("unknown or unregistered tool: {name}"),
        )
        .with_details(serde_json::Map::from_iter([(
            "tool".to_string(),
            Value::String(name.to_string()),
        )])),
    )
}

/// `error.code` field 用の分類値 — source chain の `CheaperError` code を
/// 優先し、裸の `AbortReason` はその code/name に潰す。message/payload は
/// field に載せない(metadata-only 規約)。
fn tool_error_code(error: &ToolError) -> &str {
    if let Some(e) = crate::tools::process::find_cheaper_error(error.as_ref()) {
        return e.code().as_str();
    }
    if let Some(reason) = error.downcast_ref::<AbortReason>() {
        return reason.code().unwrap_or_else(|| reason.name());
    }
    "ERROR"
}

/// `observeAccess`(TS line 130-143)。`manager.get` は missing で
/// `invalidResource` throw → Rust もそのまま `Err` 伝播。`access`/`manager`
/// を引数で受けるのは、registered tool へ渡す observer closure が
/// `RuntimeInner` 構築前に組み立てられるため(pub メソドと同一路径)。
fn observe_access_impl(
    access: &ResourceAccess,
    manager: &OperationManager,
    path: &str,
    mode: ClaimMode,
    scope: Option<ResourceScope>,
    support: bool,
) -> Result<(), ToolError> {
    let active = ACTIVE_OPERATION
        .try_with(|a| a.clone())
        .map_err(|_| invalid_resource_err("observation requires active operation"))?;
    let operation = manager
        .get(&active.id)
        .map_err(|e| Box::new(e) as ToolError)?;
    if operation.state != OperationState::Running {
        return Err(invalid_resource_err("operation has settled"));
    }
    access.observe(path, mode, scope, Some(&active), support)?;
    let operation = manager
        .get(&active.id)
        .map_err(|e| Box::new(e) as ToolError)?;
    if operation.state != OperationState::Running {
        return Err(invalid_resource_err("operation has settled"));
    }
    Ok(())
}

/// `{...definition, execute: (input, signal) => runtime.execute(name, input,
/// {signal})}`。`names` 順を保持し、`prepare_arguments` は `Arc` で共有する。
fn external_definitions(inner: &Arc<RuntimeInner>) -> Vec<CanonicalToolDefinition<'static>> {
    let weak = Arc::downgrade(inner);
    inner
        .names
        .iter()
        .filter_map(|name| {
            let entry = inner.registered.get(name)?;
            let tool_name = *name;
            let weak = weak.clone();
            Some(CanonicalToolDefinition {
                name: tool_name,
                label: entry.definition.label.clone(),
                description: entry.definition.description.clone(),
                parameters: entry.definition.parameters.clone(),
                prepare_arguments: entry.definition.prepare_arguments.clone(),
                execute: Box::new(move |input, signal| {
                    let weak = weak.clone();
                    Box::pin(async move {
                        let Some(inner) = weak.upgrade() else {
                            return Err(invalid_resource_err("tool runtime is disposed"));
                        };
                        let runtime = ToolRuntime {
                            inner,
                            definitions: Arc::new(Vec::new()),
                        };
                        runtime
                            .execute(tool_name.as_str(), &input, Some(ToolCallContext { signal }))
                            .await
                    }) as CanonicalExecuteFuture<'static>
                }),
            })
        })
        .collect()
}

/// native tool の registered definition。metadata は borrowed の
/// `*_tool_definition` から取り、`execute` は `Arc<ToolCtx>` 所有の
/// `'static` closure で組み直す(borrowed options を env に持てないため)。
fn registered_definition(
    name: ToolName,
    ctx: &Arc<ToolCtx>,
) -> Result<CanonicalToolDefinition<'static>, ToolError> {
    // `(label, description, parameters)` — borrowed definition から抽出する
    // owned metadata。`prepare_arguments` は `CanonicalToolDefinition<'a>`
    // が invariant のため borrowed def から `'static` へ move できない —
    // hook を持つのは edit のみで canonical fn(`prepare_edit_arguments`)
    // を明示的に再所有する。
    macro_rules! metadata {
        ($borrowed:expr) => {{
            let borrowed = $borrowed?;
            (borrowed.label, borrowed.description, borrowed.parameters)
        }};
    }
    let (label, description, parameters, prepare_arguments, execute) = match name {
        ToolName::Read => {
            let (label, description, parameters) = metadata!(read_tool_definition(ctx.read()));
            let ctx = Arc::clone(ctx);
            let execute: Box<crate::definition::CanonicalExecute<'static>> =
                Box::new(move |input: Value, signal: Option<AbortSignal>| {
                    let ctx = Arc::clone(&ctx);
                    Box::pin(async move {
                        crate::tools::read::create_read_tool(ctx.read())
                            .execute(&input, signal.as_ref())
                    }) as CanonicalExecuteFuture<'static>
                });
            (label, description, parameters, None, execute)
        }
        ToolName::Write => {
            let (label, description, parameters) = metadata!(write_tool_definition(
                create_write_operations(ctx.boundary())?
            ));
            let ctx = Arc::clone(ctx);
            let execute: Box<crate::definition::CanonicalExecute<'static>> =
                Box::new(move |input: Value, signal: Option<AbortSignal>| {
                    let ctx = Arc::clone(&ctx);
                    Box::pin(async move {
                        let ops = create_write_operations(ctx.boundary())?;
                        crate::tools::write::create_write_tool(ops).execute(&input, signal.as_ref())
                    }) as CanonicalExecuteFuture<'static>
                });
            (label, description, parameters, None, execute)
        }
        ToolName::Edit => {
            let (label, description, parameters) = metadata!(edit_tool_definition(
                create_edit_operations(ctx.boundary())?
            ));
            let prepare: Option<std::sync::Arc<crate::definition::CanonicalPrepare<'static>>> =
                Some(std::sync::Arc::new(
                    crate::tools::edit::prepare_edit_arguments,
                ));
            let ctx = Arc::clone(ctx);
            let execute: Box<crate::definition::CanonicalExecute<'static>> =
                Box::new(move |input: Value, signal: Option<AbortSignal>| {
                    let ctx = Arc::clone(&ctx);
                    Box::pin(async move {
                        let ops = create_edit_operations(ctx.boundary())?;
                        crate::tools::edit::create_edit_tool(ops).execute(&input, signal.as_ref())
                    }) as CanonicalExecuteFuture<'static>
                });
            (label, description, parameters, prepare, execute)
        }
        ToolName::Ls => {
            let (label, description, parameters) =
                metadata!(ls_tool_definition(create_ls_operations(ctx.boundary())?));
            let ctx = Arc::clone(ctx);
            let execute: Box<crate::definition::CanonicalExecute<'static>> =
                Box::new(move |input: Value, signal: Option<AbortSignal>| {
                    let ctx = Arc::clone(&ctx);
                    Box::pin(async move {
                        let ops = create_ls_operations(ctx.boundary())?;
                        crate::tools::ls::create_ls_tool(ops).execute(&input, signal.as_ref())
                    }) as CanonicalExecuteFuture<'static>
                });
            (label, description, parameters, None, execute)
        }
        ToolName::Grep => {
            let (label, description, parameters) = metadata!(grep_tool_definition(ctx.grep()));
            let ctx = Arc::clone(ctx);
            let execute: Box<crate::definition::CanonicalExecute<'static>> =
                Box::new(move |input: Value, signal: Option<AbortSignal>| {
                    let ctx = Arc::clone(&ctx);
                    Box::pin(async move {
                        crate::tools::grep::create_grep_tool(ctx.grep())?
                            .execute(&input, signal.as_ref())
                    }) as CanonicalExecuteFuture<'static>
                });
            (label, description, parameters, None, execute)
        }
        ToolName::Find => {
            let (label, description, parameters) = {
                let walk = ctx.walk();
                metadata!(find_tool_definition(&walk))
            };
            let ctx = Arc::clone(ctx);
            let execute: Box<crate::definition::CanonicalExecute<'static>> =
                Box::new(move |input: Value, signal: Option<AbortSignal>| {
                    let ctx = Arc::clone(&ctx);
                    Box::pin(async move {
                        let walk = ctx.walk();
                        crate::tools::find::create_find_tool(&walk).execute(&input, signal.as_ref())
                    }) as CanonicalExecuteFuture<'static>
                });
            (label, description, parameters, None, execute)
        }
        ToolName::CountTokens => {
            let (label, description, parameters) =
                metadata!(count_tokens_tool_definition(ctx.count_tokens()));
            let ctx = Arc::clone(ctx);
            let execute: Box<crate::definition::CanonicalExecute<'static>> =
                Box::new(move |input: Value, _signal: Option<AbortSignal>| {
                    let ctx = Arc::clone(&ctx);
                    Box::pin(async move {
                        crate::tools::count_tokens::create_count_tokens_tool(ctx.count_tokens())
                            .execute(&input)
                    }) as CanonicalExecuteFuture<'static>
                });
            (label, description, parameters, None, execute)
        }
        ToolName::Process => {
            // `process` の definition は `'static` — そのまま登録する。
            return Ok(process_tool_definition(ctx.process()));
        }
        ToolName::Command => {
            // `command` の definition も `'static` — そのまま登録する。
            // ここに到達するのは `names` に `Command` を含む場合のみで、
            // nonempty-grants gate + pin は `create_tool_runtime` が
            // 既に通過済み。
            return Ok(command_tool_definition(ctx.command()));
        }
    };
    Ok(CanonicalToolDefinition {
        name,
        label,
        description,
        parameters,
        prepare_arguments,
        execute,
    })
}

/// `ready()` の request/parent 経路。`OnceCell` の init closure が
/// `self` を borrow できないため `Arc<RuntimeInner>` を受け取る。
fn ready_init(
    inner: Arc<RuntimeInner>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ResourceLease, AbortReason>> + Send>>
{
    Box::pin(async move {
        // `if (controller.signal.aborted) throw controller.signal.reason`。
        if let Some(reason) = inner.controller.signal().reason() {
            return Err(reason);
        }
        if let Some(parent) = inner.options_parent_lease {
            // parent branch: scope prepare → 親 lease から preparation を
            // 借用 → `claimContains` で scope claims が親内か検証する。
            let prepared = inner
                .access
                .prepare()
                .map_err(AbortReason::from)?
                .lease_preparation();
            let parent_prep = inner
                .resources
                .parent(parent, &inner.owner_id)
                .map_err(AbortReason::from)?;
            if !prepared
                .claims
                .iter()
                .all(|claim| parent_prep.claims.iter().any(|p| claim_contains(p, claim)))
            {
                return Err(AbortReason::from(invalid_resource(
                    "scope claims are outside parent",
                )));
            }
            Ok(parent)
        } else {
            // `resources.request({ownerId, kind:"job", signal: controller.signal,
            // prepare: () => access.prepare(), start: (lease) => parentLease = lease})`。
            let request_inner = Arc::clone(&inner);
            let parent_lease = Arc::clone(&inner.parent_lease);
            inner
                .resources
                .request(ResourceRequest {
                    owner_id: inner.owner_id.clone(),
                    kind: LeaseKind::Job,
                    signal: Some(inner.controller.signal()),
                    parent_lease: None,
                    access: None,
                    prepare: Box::new(move || {
                        let prepared = request_inner
                            .access
                            .prepare()
                            .map(|scope| scope.lease_preparation())
                            .map_err(AbortReason::from);
                        Box::pin(async move { prepared })
                    }),
                    can_start: None,
                    start: move |lease| {
                        *parent_lease.lock().unwrap() = Some(lease);
                        Ok(lease)
                    },
                })
                .await
        }
    })
}

impl ToolRuntime {
    /// `catalog`(TS getter)。
    pub fn catalog(&self) -> &ToolCatalog {
        &self.inner.catalog
    }

    /// `journal`(TS getter)。
    pub fn journal(&self) -> &MutationJournal {
        &self.inner.journal
    }

    /// `budget`(readonly counter view)。
    pub fn budget(&self) -> ToolBudget {
        self.inner.accounting.budget()
    }

    /// `meter`(readonly counter view)。
    pub fn meter(&self) -> ToolMeter {
        self.inner.metering.meter()
    }

    /// `definitions`(公開向け — `execute` は `runtime.execute` へ委譲する)。
    pub fn definitions(&self) -> &[CanonicalToolDefinition<'static>] {
        &self.definitions
    }

    /// `ready()`。`readyPromise` memoization は `OnceCell` に `Result`
    /// を格納する `get_or_init` — TS 同様、reject された promise は
    /// retry せず永続 memoize される。`ready_requested` は sync 呼出し
    /// 時点で立てる(async fn の body は poll まで走らないため、
    /// `ready()` 呼出後に `execute` しても `readiness !== undefined`
    /// が観測できるようにする — TS の `const p = r.ready()` 同期代入相当)。
    pub fn ready(
        &self,
    ) -> impl std::future::Future<Output = Result<ResourceLease, AbortReason>> + '_ {
        self.inner.ready_requested.store(true, Ordering::SeqCst);
        async move { self.ready_inner().await }
    }

    async fn ready_inner(&self) -> Result<ResourceLease, AbortReason> {
        let inner = Arc::clone(&self.inner);
        self.inner
            .ready_cell
            .get_or_init(|| ready_init(inner))
            .await
            .clone()
    }

    /// `observeAccess(path, mode, scope?, support?)`(TS async → Rust sync —
    /// `access.observe` は同期処理のみ)。
    pub fn observe_access(
        &self,
        path: &str,
        mode: ClaimMode,
        scope: Option<ResourceScope>,
        support: bool,
    ) -> Result<(), ToolError> {
        observe_access_impl(
            &self.inner.access,
            &self.inner.manager,
            path,
            mode,
            scope,
            support,
        )
    }

    /// `accesses()`。
    pub fn accesses(&self) -> AccessSnapshot {
        self.inner.access.snapshot()
    }

    /// `operations()`(TS `() => manager.records(ownerId)`)。
    pub fn operations(&self) -> Vec<OperationRecord> {
        self.inner.manager.records(Some(&self.inner.owner_id))
    }

    /// `execute(name, input, context?)`(TS pipeline line 227-291)。
    ///
    /// `cheaper.tool` span + field `tool.name` は全 tool call の単一の正
    /// instrumentation 点(design §instrumentation 配置)。span 名を tool 毎に
    /// 動的化しないのは `span!` の `'static` 制約との兼ね合い — tool 名は
    /// field で機械検索可能とし、個別 tool file には emit を散らさない。
    /// field は metadata-only(`error.code` のような分類値のみ)。
    pub async fn execute(
        &self,
        name: &str,
        input: &Value,
        context: Option<ToolCallContext>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let span = tracing::info_span!("cheaper.tool", tool.name = name);
        async move {
            let result = self.execute_inner(name, input, context).await;
            match &result {
                Ok(_) => {
                    tracing::debug!(
                        target: "cheaper.tool.result",
                        message = format_args!("tool result"),
                        result.ok = true
                    );
                }
                Err(error) => {
                    tracing::debug!(
                        target: "cheaper.tool.result",
                        message = format_args!("tool result"),
                        result.ok = false,
                        error.code = tool_error_code(error)
                    );
                }
            }
            result
        }
        .instrument(span)
        .await
    }

    async fn execute_inner(
        &self,
        name: &str,
        input: &Value,
        context: Option<ToolCallContext>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let inner = &self.inner;
        if let Some(reason) = inner.controller.signal().reason() {
            return Err(throw_reason(reason));
        }
        let entry = ToolName::from_canonical(name)
            .and_then(|n| inner.registered.get(&n).cloned())
            .ok_or_else(|| unknown_tool(name))?;
        // `prepareArguments`(input 正規化)を検証前に適用する — TS では Pi
        // session 層が schema 検証前に吸収する seam で、Rust は session
        // 層を持たないため runtime 内に折り込む(definition/executor と
        // 同じ ordering)。`structuredClone(input)` 相当は prepared snapshot
        // 自体が owned `Value` であることで満たす。
        let submitted_input = match &entry.definition.prepare_arguments {
            Some(prepare) => prepare(input),
            None => input.clone(),
        };
        validate_tool_input(name, &entry.schema, &submitted_input)?;
        let readiness = self.readiness_future();
        let scoped = inner.access.explicit
            || inner.parent_lease.lock().unwrap().is_some()
            || readiness.is_some();
        let signal = match context.and_then(|c| c.signal) {
            Some(call) => AbortSignal::any(&[inner.controller.signal(), call]),
            None => inner.controller.signal(),
        };
        if let Some(reason) = signal.reason() {
            return Err(throw_reason(reason));
        }
        let mut declaration = match entry.definition.name {
            ToolName::Process => {
                let task = submitted_input
                    .get("task")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                process_operation_declaration(&inner.tool_ctx.process, task)
                    .map_err(|e| Box::new(e) as ToolError)?
            }
            ToolName::Command => {
                // `{name}` 入力の grant を declaration 段階で解決 —
                // 未知 grant は schedule 前に `POLICY_VIOLATION` で
                // fail-closed(design §(f)/(g))。
                let grant = submitted_input
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                command_operation_declaration(&inner.tool_ctx.command, grant)
                    .map_err(|e| Box::new(e) as ToolError)?
            }
            name => OperationDeclaration::new(if is_mutating_tool(name) {
                ClaimMode::Exclusive
            } else {
                ClaimMode::Shared
            }),
        };
        declaration.resource_scoped = scoped;
        declaration.parent_lease = Some(ParentLeaseSource::Resolver({
            let inner = Arc::clone(&self.inner);
            Box::new(move || *inner.parent_lease.lock().unwrap())
        }));
        declaration.readiness = readiness;
        if scoped {
            let access_scope = submitted_input.clone();
            let inner = Arc::clone(&self.inner);
            let tool_name = entry.definition.name;
            declaration.prepare = Some(Box::new(move || {
                let prepared = inner
                    .access
                    .operation(tool_name, &access_scope)
                    .map_err(AbortReason::from);
                Box::pin(async move { prepared })
            }));
        }
        let inner2 = Arc::clone(&self.inner);
        let name_owned = name.to_string();
        // `is_process` の managed-subprocess 一般化(design §(g)) —
        // `command` の spawn 済み子 process も `managed.track()` が
        // 数え、`drain()`/`wait_idle` が settle を待つ。
        let is_managed = matches!(entry.definition.name, ToolName::Process | ToolName::Command);
        // `pending = accounting.run(() => scheduler.run(...))`。
        let pending = async move {
            let _managed = if is_managed {
                Some(inner2.managed.track())
            } else {
                None
            };
            let budget_inner = Arc::clone(&inner2);
            inner2
                .accounting
                .run(move || {
                    let inner = Arc::clone(&budget_inner);
                    let entry = Arc::clone(&entry);
                    let signal = signal.clone();
                    async move {
                        let sched_inner = Arc::clone(&inner);
                        inner
                            .scheduler
                            .run(
                                CreateOperationOptions {
                                    owner_id: inner.owner_id.clone(),
                                    kind: "tool".to_string(),
                                    tool_name: Some(name_owned),
                                    resource_claims: None,
                                    signal: Some(signal),
                                },
                                declaration,
                                move |operation_signal| {
                                    let inner = Arc::clone(&sched_inner);
                                    let entry = Arc::clone(&entry);
                                    async move {
                                        let operation = inner
                                            .manager
                                            .records(Some(&inner.owner_id))
                                            .into_iter()
                                            .find(|record| {
                                                record.state == OperationState::Running
                                                    && record.signal == operation_signal
                                            })
                                            .ok_or_else(|| {
                                                invalid_resource_err("missing running operation")
                                            })?;
                                        let active = ActiveResourceAccess {
                                            id: operation.id.clone(),
                                            claims: operation
                                                .resource_claims
                                                .clone()
                                                .unwrap_or_default(),
                                        };
                                        ACTIVE_OPERATION
                                            .scope(
                                                active,
                                                inner.metering.run(move || {
                                                    (entry.definition.execute)(
                                                        submitted_input,
                                                        Some(operation_signal),
                                                    )
                                                }),
                                            )
                                            .await
                                    }
                                },
                            )
                            .await
                            .map_err(|reason| Box::new(reason) as ToolError)
                    }
                })
                .await
        };
        pending.await
    }

    /// `readyPromise !== undefined` なら shared readiness future。
    fn readiness_future(&self) -> Option<ReadinessFuture> {
        let requested = self.inner.ready_requested.load(Ordering::SeqCst)
            || self.inner.ready_cell.initialized();
        if !requested {
            return None;
        }
        let inner = Arc::clone(&self.inner);
        Some(Box::pin(async move {
            let cell_inner = Arc::clone(&inner);
            inner
                .ready_cell
                .get_or_init(move || ready_init(cell_inner))
                .await
                .clone()
                .map(|_| ())
        }))
    }

    /// `abort(reason?)`。
    pub fn abort(&self, reason: Option<AbortReason>) {
        self.inner
            .controller
            .abort(reason.unwrap_or_else(AbortReason::aborted));
    }

    /// `drain()` — abort + owner drain + managed process settlement 待機。
    pub async fn drain(&self) -> Result<(), CheaperError> {
        self.abort(None);
        self.inner
            .manager
            .drain(Some(&self.inner.owner_id), Some(self.inner.abort_grace_ms))
            .await?;
        self.inner.managed.wait_idle().await;
        Ok(())
    }

    /// `dispose()`(memoized。`disposed` promise は `OnceCell`)。
    pub async fn dispose(&self) {
        let inner = Arc::clone(&self.inner);
        let init_inner = Arc::clone(&inner);
        let _ = inner
            .disposed
            .get_or_init(move || async move { dispose_inner(init_inner).await })
            .await;
    }

    /// worker scope 消滅時に owner scope の `scopes` 集合から自分を外す
    /// hook(TS の `scope.dispose` wrap 相当)。invocation が呼ぶ。
    pub(crate) fn set_dispose_hook(&self, hook: impl Fn() + Send + Sync + 'static) {
        *self.inner.dispose_hook.lock().unwrap() = Some(Box::new(hook));
    }

    /// invocation の `scopes` tracking 用(inner identity 比較)。
    pub(crate) fn inner_key(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }
}

impl std::fmt::Debug for ToolRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRuntime")
            .field("owner_id", &self.inner.owner_id)
            .finish_non_exhaustive()
    }
}

async fn dispose_inner(inner: Arc<RuntimeInner>) {
    let runtime = ToolRuntime {
        inner: Arc::clone(&inner),
        definitions: Arc::new(Vec::new()),
    };
    let _ = runtime.drain().await;
    // `if (!opts.parentLease && parentLease) resources.release(parentLease)`。
    if inner.options_parent_lease.is_none() {
        if let Some(lease) = *inner.parent_lease.lock().unwrap() {
            inner.resources.release(lease);
        }
    }
    if let (Some(signal), Some(id)) = (&inner.external_signal, inner.external_listener) {
        signal.remove_listener(id);
    }
    inner
        .controller
        .signal()
        .remove_listener(inner.scope_listener);
    if inner.owns_managers {
        inner.manager.dispose().await;
    }
    if let Some(hook) = inner.dispose_hook.lock().unwrap().take() {
        hook();
    }
}

/// `createToolRuntime(options, binding?)`。
pub fn create_tool_runtime(
    options: ToolRuntimeOptions,
    binding: Option<OperationBinding>,
) -> Result<ToolRuntime, ToolError> {
    // `opts.process = resolveProcessPolicy(options.process)`。
    let process =
        resolve_process_policy(options.process.as_ref()).map_err(|e| Box::new(e) as ToolError)?;
    if options.names.contains(&ToolName::Process) && process.tasks.is_empty() {
        return Err(Box::new(CheaperError::new(
            ErrorCode::PolicyViolation,
            "process exposure requires nonempty authorized tasks",
        )));
    }
    // `opts.command = resolveCommandPolicy(options.command)`。
    let command =
        resolve_command_policy(options.command.as_ref()).map_err(|e| Box::new(e) as ToolError)?;
    if options.names.contains(&ToolName::Command) {
        // `command` の実効的な権限境界: trusted layer の grant が
        // 空なら exposure を fail-closed(process gate と同型、
        // design §(g))。`Command` を含まない runtime は grant が
        // ambient PATH で解決不能でも dormant として許容する。
        if command.grants.is_empty() {
            return Err(Box::new(CheaperError::new(
                ErrorCode::PolicyViolation,
                "command exposure requires nonempty authorized grants",
            )));
        }
        // writeScope × command は fail-closed reject(design §(c)) —
        // 子 process の副作用を resource 層が観測不能のため、
        // vacuous な coverage を許さない。
        if options.write_scope.is_some() {
            return Err(Box::new(CheaperError::new(
                ErrorCode::PolicyViolation,
                "command is incompatible with worker writeScope",
            )));
        }
    }
    let abort_grace_ms = options.abort_grace_ms.unwrap_or(5000.0);
    if !(abort_grace_ms.is_finite() && abort_grace_ms >= 0.0) {
        return Err(Box::new(CheaperError::new(
            ErrorCode::InvalidRequest,
            "abort grace must be a finite nonnegative number",
        )));
    }
    let journal = options
        .journal
        .clone()
        .unwrap_or_else(|| Arc::new(MutationJournal::default()));
    let accounting = create_tool_budget(ToolBudgetOptions {
        max_tool_calls: options.max_tool_calls,
        on_limit: options.on_limit,
    });
    let metering = create_tool_meter();
    let controller = AbortController::new();
    let owns_managers = binding.is_none();
    let (manager, owner_id, resources, scheduler) = match binding {
        Some(b) => (b.manager, b.owner_id, b.resources, b.scheduler),
        None => {
            let manager = OperationManager::new(OperationManagerOptions {
                signal: None,
                abort_grace_ms: Some(abort_grace_ms),
                clock: None,
            })
            .map_err(|e| Box::new(e) as ToolError)?;
            let resources = ResourceManager::new(&options.root, options.symlink_policy);
            let scheduler = ToolScheduler::new(
                manager.clone(),
                options.concurrency.as_ref(),
                Some(resources.clone()),
            )
            .map_err(|e| Box::new(e) as ToolError)?;
            (manager, "tool-runtime".to_string(), resources, scheduler)
        }
    };
    // per-scope executable pin(design §(b)/(g)) — `names` に
    // `Command` を含む場合のみ、その時点の ambient PATH で grant
    // executable を canonical path へ pin する。pin 不能・deny 該当・
    // workspace-resident の grant は runtime 構築自体を
    // `POLICY_VIOLATION` で fail させる。`Command` を expose しない
    // runtime は pin を試行せず dormant とする。
    let command_pins = if options.names.contains(&ToolName::Command) {
        pin_command_grants(
            &command,
            &resources.root,
            options.symlink_policy,
            options.denylist.as_ref(),
        )?
    } else {
        BTreeMap::new()
    };
    // `catalog`(TS `createToolCatalog({process: opts.process})` — resolved
    // policy を直接所有させる)。
    let catalog = ToolCatalog::from_resolved_policy(process.clone());
    // `inherited` readStrictness(parentLease の owner 検証付き)。
    let inherited = match options.parent_lease {
        Some(lease) => Some(
            resources
                .parent(lease, &owner_id)
                .map_err(|e| Box::new(e) as ToolError)?
                .read_strictness,
        ),
        None => None,
    };
    let scope = ResourceScopeOptions {
        resource_manifest: options.resource_manifest.clone(),
        write_scope: options.write_scope.clone(),
        read_strictness: options.read_strictness,
    };
    // `Arc` — `observeAccess` closure と `RuntimeInner` が同一 instance を
    // 共有する。
    let access = Arc::new(ResourceAccess::new(
        ScopeAccessOptions {
            scope: scope.clone(),
            root: resources.root.clone(),
            symlink_policy: options.symlink_policy,
            names: options.names.clone(),
            denylist: options.denylist.clone(),
            catalog: catalog.clone(),
        },
        inherited,
    )?);
    // `onTimeout` — process の timeout で active operation を cancel する。
    let on_timeout: OnTimeout = {
        let manager = manager.clone();
        Arc::new(move |reason: &CheaperError| {
            let Ok(active) = ACTIVE_OPERATION.try_with(|a| a.clone()) else {
                return;
            };
            if manager
                .get(&active.id)
                .map(|record| record.state == OperationState::Running)
                .unwrap_or(false)
            {
                let _ = manager.cancel(&active.id, AbortReason::from(reason));
            }
        })
    };
    // `observeAccess` — `ACTIVE_OPERATION` task_local binding +
    // `manager.get` の running 二重確認(TS line 130-143 相当)。pub メソド
    // の `observe_access(inner)` と同一規約。
    let observe: Arc<AccessObserver<'static>> = {
        let manager = manager.clone();
        let access = Arc::clone(&access);
        Arc::new(move |path, mode, scope_arg, support| {
            observe_access_impl(&access, &manager, path, mode, scope_arg, support)
        })
    };
    // scope abort → `manager.cancelOwner(ownerId, reason)`。
    let scope_listener = {
        let manager = manager.clone();
        let owner_id = owner_id.clone();
        controller.signal().on_abort(move |reason| {
            manager.cancel_owner(&owner_id, reason.clone());
        })
    };
    // external signal → runtime abort(TS `opts.signal?.addEventListener`)。
    let external_listener = options.signal.as_ref().map(|sig| {
        let controller = controller.clone();
        sig.on_abort(move |reason| {
            controller.abort(reason.clone());
        })
    });
    // `if (opts.signal?.aborted) onExternalAbort()`。
    if let Some(reason) = options.signal.as_ref().and_then(AbortSignal::reason) {
        controller.abort(reason);
    }
    let eval_spawn = match options.eval_spawn.clone() {
        Some(spawn) => spawn,
        None => EvalSpawn::current_exe().map_err(|e| Box::new(e) as ToolError)?,
    };
    let tool_ctx = Arc::new(ToolCtx {
        root: resources.root.clone(),
        symlink_policy: options.symlink_policy,
        denylist: options.denylist.clone(),
        journal: Arc::clone(&journal),
        artifacts: options.artifacts.clone(),
        owner_id: owner_id.clone(),
        scope,
        process: process.clone(),
        command,
        command_pins,
        eval_spawn,
        on_timeout: Some(on_timeout),
        observe_access: observe,
    });
    // `registered` — `opts.names.map(definitions)` + `compileSchema` pair。
    let mut registered = HashMap::with_capacity(options.names.len());
    for &name in &options.names {
        let definition = registered_definition(name, &tool_ctx)?;
        let schema = compile_schema(definition.parameters.clone(), SchemaKind::Input)
            .map_err(|e| Box::new(e) as ToolError)?;
        registered.insert(name, Arc::new(RegisteredTool { definition, schema }));
    }
    let inner = Arc::new(RuntimeInner {
        catalog,
        journal,
        access,
        registered,
        names: options.names.clone(),
        accounting,
        metering,
        controller,
        manager,
        scheduler,
        resources,
        owner_id,
        abort_grace_ms,
        owns_managers,
        options_parent_lease: options.parent_lease,
        // `let parentLease = opts.parentLease`。
        parent_lease: Arc::new(Mutex::new(options.parent_lease)),
        external_signal: options.signal.clone(),
        external_listener,
        scope_listener,
        ready_cell: OnceCell::new(),
        ready_requested: AtomicBool::new(false),
        disposed: OnceCell::new(),
        managed: ManagedProcesses::default(),
        tool_ctx,
        dispose_hook: Mutex::new(None),
    });
    let definitions = Arc::new(external_definitions(&inner));
    Ok(ToolRuntime { inner, definitions })
}

//! arm 実装語彙(§2.3 の 13 impl)の dispatch + 各 spawn/観測実装。
//!
//! - dispatch は **resolved impl 名**(`scenario.resolve_arm(arm)` —
//!   `armBase` 1 hop 解決済み)で行い、observation の `arm` field には
//!   **宣言 arm 名**を残す(v1 `runArm` parity)。
//! - 失敗系は impl ではなく fixture + `armExpect` で表現する(§2.3
//!   — 本 module の impl は成功/失敗を問わず同じ spawn・parser
//!   経路を通り、nonzero exit / `envelopeOk:false` は観測値)。
//! - spawn は全て `std::process::Command` blocking・shell 非経由
//!   (`spawn::run`)。fixture child は `current_exe()` abs path を
//!   直接 spawn(`{fixtureChild}` は `workspace::prepare` 側で
//!   process policy command 内を展開済み)。
//! - 各 product spawn の直前に fixture JSON を書き直す(v1
//!   `runCli` の `options.fakeStream` per-invocation 規約 —
//!   `CHEAPER_TEST_FAKE_STREAM` が指す file は spawn 毎に確定)。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cheaper_core::errors::CheaperError;
use serde_json::{json, Map, Value};

use crate::child::{self, ChildMode};
use crate::fixture::bench_error;
use crate::manifest::{FakeStream, ProfileSpec, Scenario};
use crate::measure::{self, Mode, Observation, ResolveObservation, RouteObservation};
use crate::spawn::{self, SpawnOptions, Spawned};
use crate::workspace::BenchWorkspace;

/// arm 実行の共有 context — spawn 先・隔離環境・宣言の参照。
pub struct ArmContext<'a> {
    /// 計測対象 product binary(dev-seam build — preflight 済み)。
    pub binary: &'a Path,
    /// observation 1 回分の隔離 workspace/cfgDir。
    pub workspace: &'a BenchWorkspace,
    /// `env::build` 済みの完全 spawn env(`armEnv` 適用済み)。
    pub env: &'a BTreeMap<String, String>,
    pub scenario: &'a Scenario,
    /// fixture child として spawn する bench bin 自身(`current_exe`)。
    pub fixture_child_exe: &'a Path,
    /// repo root(`savedWorkflowFile` resolve・command 表示用)。
    pub repo_root: &'a Path,
}

/// `resolve_arm` 済み impl 名 → observation mode(§3.3 語彙)。
fn impl_mode(resolved: &str) -> Mode {
    match resolved {
        "workflow" | "dynamic-workflow" | "saved-workflow" => Mode::Workflow,
        "direct-filesystem" => Mode::DirectToolOperation,
        "direct-worker-tool-call" => Mode::SingleDirectWorkerWithOneToolCall,
        "direct-worker-sequence" => Mode::ScriptedDirectCliSequence,
        "full-definition" => Mode::FullDefinitionRead,
        "compact-interface" => Mode::PublicInterfaceDiscovery,
        "route-resolve" => Mode::RouteResolve,
        "route-rewrite" => Mode::RouteRewrite,
        "route-run" => Mode::RouteRun,
        "route-logs" => Mode::RouteLogs,
        "direct-child-exec" => Mode::DirectChildExec,
        other => unreachable!("manifest validation guarantees impl vocabulary: {other}"),
    }
}

/// 宣言 arm 1 回分の observation を実行する — impl dispatch →
/// 観測 field 埋め → `armExpect` 照合/fallback success。
pub fn run_observation(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    repetition: u32,
    warmup: bool,
) -> Result<Observation, CheaperError> {
    let resolved = ctx.scenario.resolve_arm(declared_arm);
    let mode = impl_mode(resolved);
    let discovery = match resolved {
        "saved-workflow" => "saved-definition",
        "workflow" | "dynamic-workflow" => "generated-definition",
        "full-definition" => "private-dag-read",
        "compact-interface" => "public-interface-only",
        _ => "no-workflow-definition",
    };
    let mut observation = Observation::new(
        ctx.scenario,
        declared_arm,
        repetition,
        warmup,
        mode,
        discovery,
    );
    match resolved {
        "workflow" | "dynamic-workflow" | "saved-workflow" => {
            run_workflow(ctx, declared_arm, resolved, &mut observation)?;
        }
        "direct-filesystem" => run_direct_filesystem(ctx, &mut observation)?,
        "direct-worker-tool-call" => {
            run_direct_worker_tool_call(ctx, declared_arm, &mut observation)?;
        }
        "direct-worker-sequence" => {
            run_direct_worker_sequence(ctx, declared_arm, &mut observation)?;
        }
        "full-definition" => run_full_definition(ctx, &mut observation)?,
        "compact-interface" => {
            run_compact_interface(ctx, declared_arm, &mut observation)?;
        }
        "route-resolve" => run_route_resolve(ctx, declared_arm, &mut observation)?,
        "route-rewrite" => run_route_rewrite(ctx, declared_arm, &mut observation)?,
        "route-run" => run_route_run(ctx, declared_arm, &mut observation)?,
        "route-logs" => run_route_logs(ctx, declared_arm, &mut observation)?,
        "direct-child-exec" => run_direct_child_exec(ctx, &mut observation)?,
        other => {
            return Err(bench_error(format!("unsupported arm impl: {other}")));
        }
    }
    // `success` — `armExpect` 宣言時は key 全項目一致、未宣言は
    // fallback 規約(§2.3 — mismatch は観測値であって throw しない)。
    if let Some(expect) = ctx.scenario.arm_expect.get(declared_arm) {
        observation.expect_mismatches = measure::evaluate_expect(expect, &observation);
        observation.success = observation.expect_mismatches.is_empty();
    } else {
        observation.success = measure::fallback_success(
            observation.mode,
            observation.exit_code,
            observation.envelope_ok,
        );
    }
    Ok(observation)
}

// ---------------------------------------------------------------------------
// spawn / fixture helpers
// ---------------------------------------------------------------------------

fn spawn_product(
    ctx: &ArmContext<'_>,
    args: &[String],
    stdin: Option<&[u8]>,
) -> Result<Spawned, CheaperError> {
    spawn::run(
        ctx.binary,
        args,
        &SpawnOptions {
            cwd: ctx.workspace.path(),
            env: ctx.env,
            stdin,
        },
    )
}

/// product spawn コマンド列の helper(可読性のため `&str` から組立)。
fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn workspace_arg(ctx: &ArmContext<'_>) -> String {
    ctx.workspace.path().to_string_lossy().into_owned()
}

fn command_line(ctx: &ArmContext<'_>, args: &[String]) -> String {
    measure::display_command(ctx.binary, ctx.repo_root, args)
}

/// fixture spec 解決 — `armFakeStream[arm]` が最優先、次に per-spawn
/// profile 差し替え(`direct-worker-*` 系の `manual.responseProfiles`・
/// `toolCallPath`)、scenario `fakeStream`・v1 系
/// `responseProfiles`/`responseDelayMs` 既定の順(v1
/// `{...envInfo.fakeStream, responseProfiles:[p], toolCallPath?}` の
/// per-invocation merge 規約の再構成)。
fn fixture_spec_for(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    profile: Option<&str>,
    tool_call_path: Option<&str>,
) -> FakeStream {
    // `armFakeStream[arm]` が最優先(失敗 matrix の fixture 供給)。
    if let Some(spec) = ctx.scenario.arm_fake_stream.get(declared_arm) {
        return spec.clone();
    }
    // `{fixture:}` DSL は session 消費規則で制御するため、
    // per-spawn profile 差し替えの対象外としてそのまま使う。
    if let Some(spec @ FakeStream::Fixture { .. }) = &ctx.scenario.fake_stream {
        return spec.clone();
    }
    // per-spawn profile/toolCallPath 差し替え(direct-worker-*
    // `manual.responseProfiles[i]`・`toolCallPath` — v1 per-invocation
    // merge 規約)。
    if profile.is_some() || tool_call_path.is_some() {
        return FakeStream::Profiles(ProfileSpec {
            profiles: Some(vec![profile.unwrap_or("compact").to_string()]),
            delay_ms: Some(base_delay_ms(ctx.scenario)),
            tool_call_path: tool_call_path.map(str::to_string),
        });
    }
    // scenario `fakeStream` 宣言(`"auto"`/`{profiles}`)はそのまま、
    // 未指定は v1 `responseProfiles`/`responseDelayMs` 既定へ。
    match &ctx.scenario.fake_stream {
        Some(spec) => spec.clone(),
        None => FakeStream::Profiles(ProfileSpec {
            profiles: ctx
                .scenario
                .response_profiles
                .clone()
                .filter(|p| !p.is_empty()),
            delay_ms: Some(base_delay_ms(ctx.scenario)),
            tool_call_path: None,
        }),
    }
}

/// scenario 級 delay 既定値 — `fakeStream.profiles.delayMs` →
/// `responseDelayMs` → 0(v1 `responseDelayMs ?? 0` parity)。
fn base_delay_ms(scenario: &Scenario) -> f64 {
    if let Some(FakeStream::Profiles(spec)) = &scenario.fake_stream {
        if let Some(delay) = spec.delay_ms {
            return delay;
        }
    }
    scenario.response_delay_ms.unwrap_or(0.0)
}

fn write_fixture(ctx: &ArmContext<'_>, spec: &FakeStream) -> Result<(), CheaperError> {
    crate::fixture::write_fixture(ctx.workspace.fixture_path(), Some(spec)).map(|_| ())
}

// ---------------------------------------------------------------------------
// workflow 系 arm(workflow / dynamic-workflow / saved-workflow)
// ---------------------------------------------------------------------------

/// workflow.json の materialize — saved は `benchmarks/scenarios/`
/// から copy、dynamic/workflow は `stringify_pretty` + `"\n"` を
/// 書く(v1 `writeWorkflow` parity — definitionBytes は書いた file
/// の byte 数)。戻り値 `(definition_bytes, generation_ms)`。
fn write_workflow(ctx: &ArmContext<'_>, resolved: &str) -> Result<(u64, f64), CheaperError> {
    let path = ctx.workspace.path().join("workflow.json");
    if resolved == "saved-workflow" {
        let source = ctx.repo_root.join("benchmarks/scenarios").join(
            ctx.scenario
                .saved_workflow_file
                .as_deref()
                .ok_or_else(|| bench_error("saved-workflow arm requires savedWorkflowFile"))?,
        );
        std::fs::copy(&source, &path).map_err(|e| {
            bench_error(format!(
                "saved workflow copy failed {} → {}: {e}",
                source.display(),
                path.display()
            ))
        })?;
        let bytes = std::fs::metadata(&path)
            .map(|m| m.len())
            .map_err(|e| bench_error(format!("workflow.json stat failed: {e}")))?;
        return Ok((bytes, 0.0));
    }
    let workflow = ctx
        .scenario
        .workflow
        .as_ref()
        .ok_or_else(|| bench_error("workflow-class arm requires a workflow definition"))?;
    let started = Instant::now();
    // 計測対象の workflow definition byte — `stringify_pretty` は
    // `JSON.stringify(…, null, 2)` + `"\n"` の v1 parity 形。
    let mut text = cheaper_core::json::stringify::stringify_pretty(workflow);
    text.push('\n');
    std::fs::write(&path, &text)
        .map_err(|e| bench_error(format!("workflow.json write failed: {e}")))?;
    Ok((text.len() as u64, started.elapsed().as_secs_f64() * 1000.0))
}

fn run_workflow(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    resolved: &str,
    obs: &mut Observation,
) -> Result<(), CheaperError> {
    let (definition_bytes, generation_ms) = write_workflow(ctx, resolved)?;
    obs.workflow_definition_bytes = Some(definition_bytes);
    obs.definition_generation_ms = Some(generation_ms);

    let spec = fixture_spec_for(ctx, declared_arm, None, None);
    write_fixture(ctx, &spec)?;

    // `saved-workflow`/`dynamic-workflow` 系 arm を含む scenario は
    // `workflow interface` discovery を先に計測する(v1
    // `scenario.arms.includes(…)` — resolved impl 名で判定)。
    let needs_interface = ctx.scenario.arms.iter().any(|a| {
        matches!(
            ctx.scenario.resolve_arm(a),
            "saved-workflow" | "dynamic-workflow"
        )
    });
    if needs_interface {
        let iface_args = args(&[
            "workflow",
            "interface",
            "workflow.json",
            "--cwd",
            &workspace_arg(ctx),
            "--quiet",
        ]);
        let spawned = spawn_product(ctx, &iface_args, None)?;
        obs.interface_discovery_wall_time_ms = Some(spawned.wall_time.as_secs_f64() * 1000.0);
        let envelope = measure::parse_envelope(&spawned.stdout);
        obs.parent_visible_interface_bytes = envelope
            .as_ref()
            .and_then(measure::envelope_result)
            .map(|r| measure::json_bytes(&Value::Object(r.clone())));
    }

    let run_args = args(&[
        "workflow",
        "run",
        "workflow.json",
        "--cwd",
        &workspace_arg(ctx),
        "--quiet",
    ]);
    let spawned = spawn_product(ctx, &run_args, None)?;
    let command = command_line(ctx, &run_args);
    let envelope = measure::observe_spawn(obs, &spawned, command);
    if let Some(envelope) = envelope.as_ref() {
        measure::workflow_envelope_metrics(
            envelope,
            ctx.scenario.workflow.as_ref(),
            &mut obs.metrics,
        );
    }
    let scan = measure::scan_usage_log(ctx.workspace.config_dir())?;
    obs.direct_usage_record_count = scan.record_count;
    obs.metrics
        .insert("wallTimeMs".to_string(), json!(obs.wall_time_ms));
    Ok(())
}

// ---------------------------------------------------------------------------
// direct-filesystem(in-process floor — spawn 無し)
// ---------------------------------------------------------------------------

fn run_direct_filesystem(ctx: &ArmContext<'_>, obs: &mut Observation) -> Result<(), CheaperError> {
    let manual = ctx
        .scenario
        .manual
        .as_ref()
        .ok_or_else(|| bench_error("direct-filesystem requires a manual block"))?;
    let path = manual
        .path
        .as_deref()
        .ok_or_else(|| bench_error("direct-filesystem requires manual.path"))?;
    let target = ctx.workspace.path().join(path);
    obs.command = measure::display_in_process(&format!("fs::read({path})"));
    let started = Instant::now();
    let content = std::fs::read(&target)
        .map_err(|e| bench_error(format!("fs::read failed {}: {e}", target.display())))?;
    obs.wall_time_ms = started.elapsed().as_secs_f64() * 1000.0;
    obs.metrics.insert("nodeCount".to_string(), json!(1));
    obs.metrics.insert("workerJobCount".to_string(), json!(0));
    obs.metrics.insert("toolOnlyJobCount".to_string(), json!(0));
    obs.metrics.insert("workerTurns".to_string(), json!(0));
    obs.metrics.insert("toolCalls".to_string(), json!(0));
    obs.metrics.insert("usage".to_string(), Value::Null);
    obs.metrics
        .insert("retainedBytes".to_string(), json!(content.len()));
    obs.metrics
        .insert("materializedPromptBytes".to_string(), json!(0));
    obs.metrics
        .insert("criticalPathMs".to_string(), Value::Null);
    obs.metrics.insert("maxConcurrency".to_string(), json!(1));
    Ok(())
}

// ---------------------------------------------------------------------------
// direct-worker-tool-call(real CLI worker tool call floor)
// ---------------------------------------------------------------------------

/// `run bulk-reader` 1 spawn — fixture を書き stdin input 付きで
/// spawn し、観測値 `(spawned, envelope, metrics)` を返す
/// (direct-worker-sequence が per-task spawn に流用する共通形)。
fn spawn_bulk_reader(
    ctx: &ArmContext<'_>,
    spec: &FakeStream,
    input: &Value,
) -> Result<(Spawned, Option<Value>, measure::WorkerMetrics), CheaperError> {
    write_fixture(ctx, spec)?;
    let run_args = args(&[
        "run",
        "bulk-reader",
        "--cwd",
        &workspace_arg(ctx),
        "--quiet",
    ]);
    let stdin = serde_json::to_string(input)
        .map_err(|e| bench_error(format!("worker input serialize failed: {e}")))?;
    let spawned = spawn_product(ctx, &run_args, Some(stdin.as_bytes()))?;
    let envelope = measure::parse_envelope(&spawned.stdout);
    let metrics = measure::worker_envelope_metrics(envelope.as_ref());
    Ok((spawned, envelope, metrics))
}

fn run_direct_worker_tool_call(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    obs: &mut Observation,
) -> Result<(), CheaperError> {
    let manual = ctx
        .scenario
        .manual
        .as_ref()
        .ok_or_else(|| bench_error("direct-worker-tool-call requires a manual block"))?;
    let task = manual
        .tasks
        .first()
        .ok_or_else(|| bench_error("direct-worker-tool-call requires manual.tasks[0]"))?;
    let mut input = Map::new();
    input.insert("task".to_string(), json!(task));
    if let Some(path) = &manual.tool_call_path {
        input.insert("paths".to_string(), json!([path]));
    }
    let spec = fixture_spec_for(
        ctx,
        declared_arm,
        Some("compact"),
        manual.tool_call_path.as_deref(),
    );
    let run_args = args(&[
        "run",
        "bulk-reader",
        "--cwd",
        &workspace_arg(ctx),
        "--quiet",
    ]);
    let command = command_line(ctx, &run_args);
    let (spawned, envelope, metrics) = spawn_bulk_reader(ctx, &spec, &Value::Object(input))?;
    // observe_spawn が envelope parse を担うが、ここでは既に parse
    // 済み — 結果を直接畳む(2 度 parse しない)。
    obs.command = format!("{command} (one read tool call)");
    obs.wall_time_ms = spawned.wall_time.as_secs_f64() * 1000.0;
    obs.exit_code = spawned.exit_code;
    obs.envelope_ok = Some(measure::envelope_ok(envelope.as_ref()));
    obs.error_code = measure::envelope_error_code(envelope.as_ref());
    let result_bytes = envelope
        .as_ref()
        .and_then(|e| e.get("result"))
        .map(measure::json_bytes)
        .unwrap_or(0);
    obs.metrics.insert("nodeCount".to_string(), json!(1));
    obs.metrics.insert("workerJobCount".to_string(), json!(1));
    obs.metrics.insert("toolOnlyJobCount".to_string(), json!(0));
    obs.metrics
        .insert("workerTurns".to_string(), json!(metrics.worker_turns));
    obs.metrics
        .insert("toolCalls".to_string(), json!(metrics.tool_calls));
    measure::insert_scripted_usage(&mut obs.metrics, metrics.usage.clone());
    obs.metrics
        .insert("retainedBytes".to_string(), json!(result_bytes));
    obs.metrics
        .insert("materializedPromptBytes".to_string(), Value::Null);
    obs.metrics
        .insert("criticalPathMs".to_string(), Value::Null);
    obs.metrics.insert("maxConcurrency".to_string(), json!(1));
    obs.metrics
        .insert("repairs".to_string(), json!(metrics.repairs));
    obs.metrics.insert(
        "transientRetries".to_string(),
        json!(metrics.transient_retries),
    );
    obs.metrics.insert(
        "postprocessFailures".to_string(),
        if obs.error_code.as_deref() == Some("INVALID_OUTPUT") {
            json!(metrics.repairs)
        } else {
            Value::Null
        },
    );
    obs.metrics.insert(
        "cliReportedDurationMs".to_string(),
        metrics.duration_ms.map_or(Value::Null, |v| json!(v)),
    );
    let scan = measure::scan_usage_log(ctx.workspace.config_dir())?;
    obs.direct_usage_record_count = scan.record_count;
    obs.direct_invocation_turns_proxy = Some(1);
    Ok(())
}

// ---------------------------------------------------------------------------
// direct-worker-sequence(逐次 `run bulk-reader` — file/child → hints)
// ---------------------------------------------------------------------------

/// fixture child 直接 spawn(`manual.child`/`route` 宣言の mode・
/// bytes で bench bin を起こす — route を通さない child process
/// floor でも同一 spawn 経路を使う)。
fn spawn_fixture_child(
    ctx: &ArmContext<'_>,
    mode: ChildMode,
    bytes: Option<u64>,
) -> Result<Spawned, CheaperError> {
    let mut child_env = ctx.env.clone();
    if let Some(bytes) = bytes {
        child_env.insert(child::ENV_CHILD_BYTES.to_string(), bytes.to_string());
    }
    spawn::run(
        ctx.fixture_child_exe,
        &args(&[child::FIXTURE_CHILD_ARGV, mode.name()]),
        &SpawnOptions {
            cwd: ctx.workspace.path(),
            env: &child_env,
            stdin: None,
        },
    )
}

fn run_direct_worker_sequence(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    obs: &mut Observation,
) -> Result<(), CheaperError> {
    let manual = ctx
        .scenario
        .manual
        .as_ref()
        .ok_or_else(|| bench_error("direct-worker-sequence requires a manual block"))?;
    if manual.tasks.is_empty() {
        return Err(bench_error("direct-worker-sequence requires manual.tasks"));
    }
    let mut hints: Vec<String> = Vec::new();
    let mut retained_bytes: u64 = 0;
    let mut process_wall_ms = 0.0f64;
    let mut read_paths: Vec<String> = Vec::new();
    if let Some(path) = &manual.read_path {
        read_paths.push(path.clone());
    }
    read_paths.extend(manual.read_paths.iter().cloned());
    let mut direct_tool_operations = 0u64;
    for path in &read_paths {
        let content = std::fs::read(ctx.workspace.path().join(path))
            .map_err(|e| bench_error(format!("direct-worker-sequence read failed {path}: {e}")))?;
        direct_tool_operations += 1;
        retained_bytes += content.len() as u64;
        hints.push(String::from_utf8_lossy(&content).into_owned());
    }
    if let Some(child_spec) = &manual.child {
        let mode = ChildMode::parse(&child_spec.mode).ok_or_else(|| {
            bench_error(format!(
                "manual.child.mode is not a known mode: {}",
                child_spec.mode
            ))
        })?;
        let spawned = spawn_fixture_child(ctx, mode, child_spec.bytes)?;
        retained_bytes += spawned.stdout.len() as u64;
        process_wall_ms += spawned.wall_time.as_secs_f64() * 1000.0;
        hints.push(String::from_utf8_lossy(&spawned.stdout).into_owned());
    }

    let run_args = args(&[
        "run",
        "bulk-reader",
        "--cwd",
        &workspace_arg(ctx),
        "--quiet",
    ]);
    obs.command = format!(
        "{} (one invocation per manual task)",
        command_line(ctx, &run_args)
    );
    let mut prior_results: Vec<String> = Vec::new();
    let mut total_wall_ms = process_wall_ms;
    let mut usage_acc = Map::new();
    let mut worker_turns = 0.0f64;
    let mut tool_calls = 0.0f64;
    let mut repairs = 0.0f64;
    let mut transient_retries = 0.0f64;
    let mut child_invocations: Vec<Value> = Vec::new();
    let mut last_envelope: Option<Value> = None;
    let mut last_exit_code: Option<i32> = None;
    let mut any_error_code: Option<String> = None;
    let mut all_ok = true;
    for (index, task) in manual.tasks.iter().enumerate() {
        let profile = manual
            .response_profiles
            .as_ref()
            .and_then(|p| p.get(index))
            .map(String::as_str)
            .unwrap_or("compact");
        let task_text = if manual.forward_full_results.unwrap_or(false) && !prior_results.is_empty()
        {
            format!(
                "{task}\nPrior full Worker results:\n{}",
                prior_results.join("\n")
            )
        } else {
            task.clone()
        };
        let mut input = Map::new();
        input.insert("task".to_string(), json!(task_text));
        if let Some(path) = &manual.tool_call_path {
            input.insert("paths".to_string(), json!([path]));
        }
        if !hints.is_empty() {
            input.insert("hints".to_string(), json!(hints));
        }
        // `toolCallPath` は最初の invocation のみに付与する(v1
        // `index === 0 ? toolCallPath : undefined` parity — 後続
        // turn は file 内容を既に hints として保持している)。
        let spec = fixture_spec_for(
            ctx,
            declared_arm,
            Some(profile),
            if index == 0 {
                manual.tool_call_path.as_deref()
            } else {
                None
            },
        );
        let (spawned, envelope, metrics) = spawn_bulk_reader(ctx, &spec, &Value::Object(input))?;
        total_wall_ms += spawned.wall_time.as_secs_f64() * 1000.0;
        last_exit_code = spawned.exit_code;
        let ok = measure::envelope_ok(envelope.as_ref());
        all_ok &= ok;
        if !ok && any_error_code.is_none() {
            any_error_code = measure::envelope_error_code(envelope.as_ref());
        }
        worker_turns += metrics.worker_turns;
        tool_calls += metrics.tool_calls;
        repairs += metrics.repairs;
        transient_retries += metrics.transient_retries;
        measure::add_usage(&mut usage_acc, metrics.usage.as_ref());
        let output_bytes = envelope
            .as_ref()
            .and_then(|e| e.get("result"))
            .map(measure::json_bytes)
            .unwrap_or(0);
        retained_bytes += output_bytes;
        child_invocations.push(json!({
            "index": index,
            "wallTimeMs": spawned.wall_time.as_secs_f64() * 1000.0,
            "promptBytes": Value::Null,
            "toolCalls": metrics.tool_calls,
            "turns": metrics.worker_turns,
            "usage": metrics.usage,
        }));
        if let Some(envelope) = &envelope {
            prior_results.push(envelope.get("result").map_or_else(
                || "null".to_string(),
                cheaper_core::json::stringify::stringify,
            ));
        }
        last_envelope = envelope;
    }
    obs.wall_time_ms = total_wall_ms;
    obs.envelope_ok = Some(all_ok);
    obs.error_code = any_error_code;
    // exitCode は最後に実行した spawn の実値を残す(複数 spawn の
    // 要約 observation なので、捏造した合成値は入れない)。
    obs.exit_code = last_exit_code;
    obs.direct_invocation_turns_proxy = Some(manual.tasks.len() as u64);
    let scan = measure::scan_usage_log(ctx.workspace.config_dir())?;
    obs.direct_usage_record_count = scan.record_count;
    obs.metrics
        .insert("nodeCount".to_string(), json!(manual.tasks.len() as f64));
    obs.metrics.insert(
        "workerJobCount".to_string(),
        json!(manual.tasks.len() as f64),
    );
    obs.metrics.insert(
        "toolOnlyJobCount".to_string(),
        json!(read_paths.len() as u64 + u64::from(manual.child.is_some())),
    );
    obs.metrics.insert(
        "directToolOperations".to_string(),
        json!(direct_tool_operations),
    );
    obs.metrics
        .insert("workerTurns".to_string(), json!(worker_turns));
    obs.metrics
        .insert("toolCalls".to_string(), json!(tool_calls));
    measure::insert_scripted_usage(&mut obs.metrics, Some(usage_acc));
    obs.metrics
        .insert("retainedBytes".to_string(), json!(retained_bytes));
    obs.metrics
        .insert("materializedPromptBytes".to_string(), Value::Null);
    obs.metrics
        .insert("criticalPathMs".to_string(), Value::Null);
    obs.metrics.insert("maxConcurrency".to_string(), json!(1));
    obs.metrics.insert("repairs".to_string(), json!(repairs));
    obs.metrics
        .insert("transientRetries".to_string(), json!(transient_retries));
    obs.metrics.insert(
        "postprocessFailures".to_string(),
        if obs.error_code.as_deref() == Some("INVALID_OUTPUT") {
            json!(repairs)
        } else {
            Value::Null
        },
    );
    obs.metrics
        .insert("childInvocations".to_string(), json!(child_invocations));
    if let Some(envelope) = &last_envelope {
        if let Some(result) = envelope.get("result") {
            obs.metrics.insert(
                "finalResultBytes".to_string(),
                json!(measure::json_bytes(result)),
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// full-definition / compact-interface(definition read vs interface)
// ---------------------------------------------------------------------------

fn run_full_definition(ctx: &ArmContext<'_>, obs: &mut Observation) -> Result<(), CheaperError> {
    let (definition_bytes, _generation) = write_workflow(ctx, "full-definition")?;
    obs.workflow_definition_bytes = Some(definition_bytes);
    obs.command = measure::display_in_process("fs::read(workflow.json)");
    let started = Instant::now();
    let content = std::fs::read(ctx.workspace.path().join("workflow.json"))
        .map_err(|e| bench_error(format!("workflow.json read failed: {e}")))?;
    obs.wall_time_ms = started.elapsed().as_secs_f64() * 1000.0;
    obs.metrics
        .insert("definitionReadBytes".to_string(), json!(content.len()));
    Ok(())
}

fn run_compact_interface(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    obs: &mut Observation,
) -> Result<(), CheaperError> {
    let (definition_bytes, _generation) = write_workflow(ctx, "compact-interface")?;
    obs.workflow_definition_bytes = Some(definition_bytes);
    let spec = fixture_spec_for(ctx, declared_arm, None, None);
    write_fixture(ctx, &spec)?;
    let iface_args = args(&[
        "workflow",
        "interface",
        "workflow.json",
        "--cwd",
        &workspace_arg(ctx),
        "--quiet",
    ]);
    let spawned = spawn_product(ctx, &iface_args, None)?;
    let command = command_line(ctx, &iface_args);
    let envelope = measure::observe_spawn(obs, &spawned, command);
    obs.parent_visible_interface_bytes = envelope
        .as_ref()
        .and_then(measure::envelope_result)
        .map(|r| measure::json_bytes(&Value::Object(r.clone())));
    obs.metrics
        .insert("interfaceBytes".to_string(), json!(spawned.stdout.len()));
    Ok(())
}

// ---------------------------------------------------------------------------
// route 系 arm
// ---------------------------------------------------------------------------

fn route_spec<'a>(ctx: &'a ArmContext<'a>) -> Result<&'a crate::manifest::RouteSpec, CheaperError> {
    ctx.scenario
        .route
        .as_ref()
        .ok_or_else(|| bench_error("route-* arm requires a route block"))
}

fn run_route_resolve(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    obs: &mut Observation,
) -> Result<(), CheaperError> {
    let route = route_spec(ctx)?;
    let spec = fixture_spec_for(ctx, declared_arm, None, None);
    write_fixture(ctx, &spec)?;
    let resolve_args = args(&[
        "route",
        "resolve",
        "--command",
        &route.command,
        "--cwd",
        &workspace_arg(ctx),
        "--quiet",
    ]);
    let spawned = spawn_product(ctx, &resolve_args, None)?;
    let command = command_line(ctx, &resolve_args);
    let envelope = measure::observe_spawn(obs, &spawned, command);
    let result = envelope
        .as_ref()
        .and_then(measure::envelope_result)
        .cloned()
        .unwrap_or_default();
    obs.resolve = Some(ResolveObservation {
        matched: result.get("matched").and_then(Value::as_bool),
        route: result
            .get("route")
            .and_then(Value::as_str)
            .map(str::to_string),
        viability: result
            .get("viability")
            .and_then(Value::as_str)
            .map(str::to_string),
        executable: result
            .get("executable")
            .and_then(Value::as_str)
            .map(str::to_string),
        suppressed: result
            .get("suppressed")
            .and_then(Value::as_str)
            .map(str::to_string),
    });
    Ok(())
}

/// `<cfg>/hook-eval/` を削除する — `evaluate_dedup` が fingerprint
/// 単位で verdict を 60s TTL cache するため、route-rewrite arm の
/// 各 spawn 前に wipe して cache hit による未 cache `route-resolve`
/// との非対称 bias を防ぐ(§2.3)。observation 毎に fresh cfgDir を
/// 取るため実効は多 spawn 同一 observation への防御。
fn wipe_hook_eval(config_dir: &Path) -> Result<(), CheaperError> {
    let dir = config_dir.join("hook-eval");
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(bench_error(format!(
            "hook-eval wipe failed {}: {e}",
            dir.display()
        ))),
    }
}

fn run_route_rewrite(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    obs: &mut Observation,
) -> Result<(), CheaperError> {
    let route = route_spec(ctx)?;
    wipe_hook_eval(ctx.workspace.config_dir())?;
    let spec = fixture_spec_for(ctx, declared_arm, None, None);
    write_fixture(ctx, &spec)?;
    let hook_args = args(&["hook", "pre-tool-use", "--target", "pretooluse"]);
    let stdin = json!({
        "tool_name": "Bash",
        "tool_input": {"command": route.command},
    });
    let stdin = serde_json::to_string(&stdin)
        .map_err(|e| bench_error(format!("hook input serialize failed: {e}")))?;
    let spawned = spawn_product(ctx, &hook_args, Some(stdin.as_bytes()))?;
    obs.command = command_line(ctx, &hook_args);
    obs.wall_time_ms = spawned.wall_time.as_secs_f64() * 1000.0;
    obs.exit_code = spawned.exit_code;
    // envelope 非出力 arm(hook response JSON)— envelopeOk/errorCode
    // は null 意味論のまま。
    let rewritten = measure::parse_hook_rewrite(&spawned.stdout);
    obs.route = Some(RouteObservation {
        rewritten: Some(rewritten.is_some()),
        payload_bytes: rewritten
            .as_deref()
            .and_then(|c| c.split_whitespace().last())
            .map(|token| token.len() as u64),
        ..RouteObservation::default()
    });
    Ok(())
}

/// `route run --payload <tok>` 1 spawn — mint → spawn → trailer
/// parse → usage scan までの共通形(route-logs setup が計測外で
/// 同じ経路を使う)。戻り値は spawn 観測値 + trailer + route record。
fn spawn_route_run(
    ctx: &ArmContext<'_>,
    route: &crate::manifest::RouteSpec,
    spec: &FakeStream,
) -> Result<RouteRunOutcome, CheaperError> {
    let token = measure::mint_route_payload(route, ctx.workspace.path(), ctx.env)?;
    write_fixture(ctx, spec)?;
    let run_args = args(&[
        "route",
        "run",
        "--payload",
        &token,
        "--cwd",
        &workspace_arg(ctx),
        "--quiet",
    ]);
    let spawned = spawn_product(ctx, &run_args, None)?;
    let trailer = measure::parse_route_trailer(&spawned.stdout);
    let scan = measure::scan_usage_log(ctx.workspace.config_dir())?;
    Ok(RouteRunOutcome {
        token,
        args: run_args,
        spawned,
        trailer,
        scan,
    })
}

struct RouteRunOutcome {
    token: String,
    args: Vec<String>,
    spawned: Spawned,
    trailer: Option<(String, bool)>,
    scan: measure::UsageScan,
}

fn run_route_run(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    obs: &mut Observation,
) -> Result<(), CheaperError> {
    let route = route_spec(ctx)?;
    let spec = fixture_spec_for(ctx, declared_arm, None, None);
    let outcome = spawn_route_run(ctx, route, &spec)?;
    obs.command = command_line(ctx, &outcome.args);
    obs.wall_time_ms = outcome.spawned.wall_time.as_secs_f64() * 1000.0;
    obs.exit_code = outcome.spawned.exit_code;
    obs.direct_usage_record_count = outcome.scan.record_count;
    let (run_id, truncated) = match &outcome.trailer {
        Some((id, t)) => (Some(id.clone()), Some(*t)),
        None => (None, None),
    };
    let route_record = outcome.scan.route.as_ref();
    let log_ref = run_id
        .clone()
        .or_else(|| route_record.and_then(|r| r.log_ref.clone()));
    let child_output_bytes = log_ref
        .as_deref()
        .and_then(|id| measure::route_log_bytes(ctx.workspace.config_dir(), id));
    obs.route = Some(RouteObservation {
        payload_bytes: Some(outcome.token.len() as u64),
        rewritten: None,
        run_id: log_ref,
        condense_fired: route_record.map(|r| r.worker_nonempty),
        truncated,
        original_tokens: route_record.and_then(|r| r.original_tokens),
        returned_tokens: route_record.and_then(|r| r.returned_tokens),
        child_output_bytes,
        returned_body_bytes: Some(outcome.spawned.stdout.len() as u64),
        log_refetch_bytes: None,
    });
    obs.metrics.insert(
        "logRefetches".to_string(),
        json!(outcome.scan.log_refetches),
    );
    obs.metrics.insert(
        "postprocessFailures".to_string(),
        route_record
            .and_then(|r| r.postprocess_failures)
            .map_or(Value::Null, |v| json!(v)),
    );
    // `kind:"route"` record `exitCode` — child process の exit code
    // mirror(route-child-nonzero-exit の観測軸。route block 語彙に
    // 対応 field が無いため metrics 側へ記録する — §3.3 の pin
    // 語彙を広げず unpinned data として残す)。
    obs.metrics.insert(
        "routeChildExitCode".to_string(),
        route_record
            .and_then(|r| r.exit_code)
            .map_or(Value::Null, |v| json!(v)),
    );
    Ok(())
}

fn run_route_logs(
    ctx: &ArmContext<'_>,
    declared_arm: &str,
    obs: &mut Observation,
) -> Result<(), CheaperError> {
    let route = route_spec(ctx)?;
    let spec = fixture_spec_for(ctx, declared_arm, None, None);
    // setup(計測外)— `route run --payload` で runId を確定させる
    // (trailer regex fallback: `kind:"route"` record `logRef`)。
    let setup = spawn_route_run(ctx, route, &spec)?;
    let (trailer_run_id, setup_truncated) = match setup.trailer {
        Some((id, truncated)) => (Some(id), Some(truncated)),
        None => (None, None),
    };
    let setup_route = setup.scan.route;
    let run_id = trailer_run_id
        .or_else(|| setup_route.as_ref().and_then(|r| r.log_ref.clone()))
        .ok_or_else(|| {
            bench_error(format!(
                "route-logs setup run produced no runId (stderr: {})",
                setup.spawned.stderr_lossy()
            ))
        })?;
    let child_output_bytes = measure::route_log_bytes(ctx.workspace.config_dir(), &run_id);
    // 計測 spawn — `route logs <runId>`。
    let logs_args = args(&[
        "route",
        "logs",
        &run_id,
        "--cwd",
        &workspace_arg(ctx),
        "--quiet",
    ]);
    let spawned = spawn_product(ctx, &logs_args, None)?;
    let command = command_line(ctx, &logs_args);
    let envelope = measure::observe_spawn(obs, &spawned, command);
    let data_bytes = envelope
        .as_ref()
        .and_then(measure::envelope_result)
        .and_then(|r| r.get("data"))
        .and_then(Value::as_str)
        .map(|s| s.len() as u64);
    // scan は setup 済み route record + 計測分 refetch record を含む。
    let scan = measure::scan_usage_log(ctx.workspace.config_dir())?;
    obs.direct_usage_record_count = scan.record_count;
    obs.metrics
        .insert("logRefetches".to_string(), json!(scan.log_refetches));
    obs.metrics.insert(
        "routeChildExitCode".to_string(),
        setup_route
            .as_ref()
            .and_then(|r| r.exit_code)
            .map_or(Value::Null, |v| json!(v)),
    );
    obs.route = Some(RouteObservation {
        payload_bytes: Some(setup.token.len() as u64),
        rewritten: None,
        run_id: Some(run_id),
        condense_fired: setup_route.as_ref().map(|r| r.worker_nonempty),
        truncated: setup_truncated,
        original_tokens: setup_route.as_ref().and_then(|r| r.original_tokens),
        returned_tokens: setup_route.as_ref().and_then(|r| r.returned_tokens),
        child_output_bytes,
        returned_body_bytes: None,
        log_refetch_bytes: data_bytes.or(Some(scan.refetch_bytes_read)),
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// direct-child-exec(fixture child process floor)
// ---------------------------------------------------------------------------

fn run_direct_child_exec(ctx: &ArmContext<'_>, obs: &mut Observation) -> Result<(), CheaperError> {
    // mode/bytes — route 宣言があれば route.childMode/childBytes、
    // 無ければ manual.child(両方無い scenario では emit 既定)。
    let (mode, bytes) = if let Some(route) = &ctx.scenario.route {
        (
            ChildMode::parse(&route.child_mode).ok_or_else(|| {
                bench_error(format!(
                    "route.childMode is not a known mode: {}",
                    route.child_mode
                ))
            })?,
            route.child_bytes,
        )
    } else {
        let child = ctx.scenario.manual.as_ref().and_then(|m| m.child.as_ref());
        (
            child
                .and_then(|c| ChildMode::parse(&c.mode))
                .unwrap_or(ChildMode::Emit),
            child.and_then(|c| c.bytes),
        )
    };
    let spawned = spawn_fixture_child(ctx, mode, bytes)?;
    obs.command = format!(
        "{} {} {}",
        ctx.fixture_child_exe.display(),
        child::FIXTURE_CHILD_ARGV,
        mode.name()
    );
    obs.wall_time_ms = spawned.wall_time.as_secs_f64() * 1000.0;
    obs.exit_code = spawned.exit_code;
    obs.route = Some(RouteObservation {
        child_output_bytes: Some(spawned.stdout.len() as u64),
        ..RouteObservation::default()
    });
    Ok(())
}

/// fixture child exe 解決 — harness 自身(`current_exe`)。fixture
/// child dispatch は `main` 側が `--fixture-child` で分岐済み。
pub fn fixture_child_exe() -> Result<PathBuf, CheaperError> {
    std::env::current_exe().map_err(|e| bench_error(format!("current_exe resolution failed: {e}")))
}

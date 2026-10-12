//! Session seam — Rig type を露出しない session 差し替え境界。
//! `SessionSpec`/`WorkerSession`/`SessionFactory`/
//! `CompletionModelFactory`/`RigSessionFactory`。

use std::sync::Arc;

use cheaper_core::errors::CheaperError;
use cheaper_core::worker::schema::CompiledSchema;
use cheaper_rig_adapter::{
    CompletionModel, RigSession, RunSnapshot, RunStream, WorkerSessionSpec,
    DEFAULT_INVALID_TOOL_RETRIES, DEFAULT_MAX_TURNS,
};
use cheaper_tools::operation::AbortSignal;
use cheaper_tools::registry::ToolRuntime;

use crate::auth::ResolvedAuth;
use crate::catalog::CatalogModel;

// ---------------------------------------------------------------------------
// Session seam — Rig type を露出しない session 差し替え境界
// ---------------------------------------------------------------------------

/// session 生成入力(TS `createWorkerSession` options 相当)。repair
/// session では `tool_runtime: None`。`output_schema` は main/repair
/// 双方で worker output schema を渡す(repair では repair schema)。
///
/// `output_schema` は provider 側 json_schema mode 等への hint —
/// `CompletionModel` 生成経路(`CompletionModelFactory`)でのみ使われ、
/// adapter の `WorkerSessionSpec` には載らない(Rig 側の配線は
/// adapter/provider の責務、未配線でも prompt + 回数上限が contract を
/// 担保する)。TS に該当 option はなく Rust 側の拡張 seam(F8)。
pub struct SessionSpec<'a> {
    /// 解決済み catalog model。
    pub model: &'a CatalogModel,
    /// 解決済み auth(api key / headers)。未設定 provider では `None`。
    pub auth: Option<&'a ResolvedAuth>,
    /// system prompt(`build_system_prompt` or repair prompt 生成済み)。
    pub system_prompt: String,
    /// tool runtime binding。`None` = tool-less session(repair)。
    pub tool_runtime: Option<ToolRuntime>,
    /// provider 向け structured-output hint。
    pub output_schema: Option<&'a CompiledSchema>,
    /// `policy.output.maxTokens`。
    pub max_output_tokens: u64,
    /// `policy.retry.transientAttempts`。
    pub transient_retries: usize,
    /// `policy.execution.maxTurns` — runtime 側の正規 turn 上限。
    /// Rig の `max_turns` は runaway 対策の内部 guard であり、cheaper
    /// の maxTurns abort(T3 が stream event から分類)より先に発火して
    /// はいけないため、factory は `max(internal_max_turns, policy_max_
    /// turns)` を内部 budget に使う(F7)。
    pub policy_max_turns: u64,
}

/// `session.prompt`/`session.abort`/`collector.snapshot` を束ねた
/// run 経路の session 面。`RigSession` 以外にも scripted fake を
/// 差せるよう runtime 側で trait 化する。
pub trait WorkerSession: Send + Sync {
    /// `session.prompt()` 相当 — streaming run の唯一の実行面。
    /// `signal` は run 集約 abort signal。
    fn stream_run(&self, prompt: String, signal: Option<AbortSignal>) -> RunStream;
    /// `session.abort()` 相当の session-local cancel 要求。signal 経路で
    /// 既に abort が伝播する実装(`RigSession`)では no-op でよい —
    /// `prompt_with_abort_grace` が grace 計測の trigger として呼ぶ。
    fn abort(&self);
    /// session accumulator snapshot(main + repair で個別に畳む)。
    fn snapshot(&self) -> RunSnapshot;
}

/// `deps.sessionFactory` 相当 — provider/model 差し替え seam。
/// runtime 呼び出し側は opaque session を受け取り、Rig type を触らない。
pub trait SessionFactory: Send + Sync {
    fn create_session(
        &self,
        spec: &SessionSpec<'_>,
    ) -> Result<Arc<dyn WorkerSession>, CheaperError>;
}

/// provider 固有の model 構築(`CompletionModel::from_rig` 呼び出しは
/// provider/plugin 実装側の責務 — runtime は opaque `CompletionModel`
/// のみを受け取る)。
pub trait CompletionModelFactory: Send + Sync {
    fn completion_model(&self, spec: &SessionSpec<'_>) -> Result<CompletionModel, CheaperError>;
}

/// production session factory — `CompletionModelFactory` で opaque model
/// を得て `RigSession::build` に渡す。
pub struct RigSessionFactory<F> {
    pub model_factory: F,
    /// agent 内部 turn guard(`DEFAULT_MAX_TURNS`)。
    pub internal_max_turns: usize,
    /// invalid tool call の retry 上限(`DEFAULT_INVALID_TOOL_RETRIES`)。
    pub invalid_tool_retries: usize,
}

impl<F> RigSessionFactory<F> {
    pub fn new(model_factory: F) -> Self {
        Self {
            model_factory,
            internal_max_turns: DEFAULT_MAX_TURNS,
            invalid_tool_retries: DEFAULT_INVALID_TOOL_RETRIES,
        }
    }
}

impl<F: CompletionModelFactory> SessionFactory for RigSessionFactory<F> {
    fn create_session(
        &self,
        spec: &SessionSpec<'_>,
    ) -> Result<Arc<dyn WorkerSession>, CheaperError> {
        let model = self.model_factory.completion_model(spec)?;
        let internal_max_turns = self.effective_internal_max_turns(spec.policy_max_turns);
        Ok(Arc::new(RigSession::build(WorkerSessionSpec {
            model,
            system_prompt: spec.system_prompt.clone(),
            tool_runtime: spec.tool_runtime.clone(),
            max_output_tokens: spec.max_output_tokens,
            max_turns: internal_max_turns,
            // F6: Rig の invalid-tool retry は model-call budget
            // (`max_turns`)を消費する — retry 上限が turn guard 未満だと
            // `invalid_tool_call_retries` 枯渇の PROVIDER_FAILURE が
            // cheaper の `maxTurns` 分類(`MAX_TURNS_EXCEEDED` abort)より
            // 先に発火し得る。Pi には invalid-call 固有の予算が無く
            // turn 数のみが上限なので、内部 guard で floor する。
            invalid_tool_retries: self.invalid_tool_retries.max(internal_max_turns),
            transient_retries: spec.transient_retries,
        })))
    }
}

impl<F> RigSessionFactory<F> {
    /// Rig 内部 turn guard の実効値 — `max(internal_max_turns,
    /// policy_max_turns)`(F7)。policy maxTurns が guard を上回ると
    /// runaway 対策が cheaper の maxTurns 分類(`MAX_TURNS_EXCEEDED`
    /// abort)より先に発火し、error code・event 列ともに TS からずれる。
    fn effective_internal_max_turns(&self, policy_max_turns: u64) -> usize {
        let policy = usize::try_from(policy_max_turns).unwrap_or(usize::MAX);
        self.internal_max_turns.max(policy)
    }
}

impl WorkerSession for RigSession {
    fn stream_run(&self, prompt: String, signal: Option<AbortSignal>) -> RunStream {
        RigSession::stream_run(self, prompt, signal)
    }

    /// `RigSession` の cancel は `stream_run` の signal select が担う
    /// (agent は stateless per-prompt で stream drop が run cancel 相当)。
    /// `session.abort()` は promptWithAbortGrace の trigger として呼ば
    /// れるためここでは no-op。
    fn abort(&self) {}

    fn snapshot(&self) -> RunSnapshot {
        RigSession::snapshot(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F7: 内部 guard は `max(DEFAULT_MAX_TURNS, policy maxTurns)`。
    /// `model_factory` は session 構築時にしか使われないため `()` で
    /// 構造体だけ作り formula だけを検証する。
    #[test]
    fn internal_max_turns_is_at_least_policy_max_turns() {
        let factory = RigSessionFactory::new(());
        assert_eq!(factory.effective_internal_max_turns(10), 1024);
        assert_eq!(factory.effective_internal_max_turns(1024), 1024);
        assert_eq!(factory.effective_internal_max_turns(2048), 2048);
        assert_eq!(factory.effective_internal_max_turns(u64::MAX), usize::MAX);
    }
}

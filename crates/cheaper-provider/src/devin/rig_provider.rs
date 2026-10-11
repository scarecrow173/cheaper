//! Rig 0.43 接着 — Devin `GetChatMessage` を custom
//! `Wire`/`Decoder`/`Transport` として実装し、
//! `cheaper_rig_adapter::CompletionModel::from_rig` へ渡す
//! `DynModel<Completion>` を組み立てる。
//!
//! - `DevinWire::encode` は request を protobuf payload へ pure に encode
//!   する（JWT mint は transport の責務 — `encode` は副作用を持てない
//!   ため metadata の field 21 は transport が append する）
//! - `DevinTransport` は `GetUserJwt`（cache 付き）→ Connect frame POST
//!   → `deframe_connect` で frame 列化
//! - `DevinDecoder` は `ChatEvent` を Rig の part handle
//!   （`TextPart`/`ReasoningPart`/`CallPart`）へ流し込み、EOS trailer で
//!   `out.end(Finish)` する。EOS 未到達の自然終端は `ProviderError::
//!   Truncated`（TS `!sawEos → error` 相当）

use std::sync::Arc;

use bytes::Bytes;
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_rig_adapter::CompletionModel;
use cheaper_runtime::auth::{ProcessEnv, ProviderEnv, ResolvedAuth};
use cheaper_runtime::run::{CompletionModelFactory, SessionSpec};
use rig::completion::{FinishReason, Usage};
use rig::driver::{Exchange, Model, Opened, Opening, Transport};
use rig::error::{EncodeError, ProviderError};
use rig::http_client::{self, DynHttpClient, HttpClientExt, Request, StatusCode};
use rig::message::{CallId, ToolName};
use rig::operation::{CallPart, Completion, Finish, ReasoningPart, Seal, TextPart};
use rig::wire::{
    Decoder, Descriptor, Flow, Mode, Out, Request as WireRequest, Secret, Wire, WireEvent,
};
use rig::ProviderResponseError;

use crate::devin::context_map::map_request_to_chat;
use crate::devin::jwt::{self, JwtError};
use crate::devin::stream::{
    build_get_chat_message_request_parts, decode_chat_frame, deframe_connect, session_ids,
    ChatEvent, ChatFinishReason, ChatRequestArgs, ChatUsage, DeframeError, DevinFrame,
    CHAT_ENDPOINT,
};

/// provider descriptor 名（`describe().name` / telemetry）。
const PROVIDER_NAME: &str = "devin";
/// plugin 宣言 api kind（`PluginProviderRegistration.api` と一致）。
pub const API_KIND: &str = "devin-local";
/// protocol 固定の既定 endpoint（catalog 非依存）。
pub const DEFAULT_HOST: &str = "https://server.codeium.com";
/// `options.env.DEVIN_API_SERVER_URL` 相当の override。
pub const HOST_ENV: &str = "DEVIN_API_SERVER_URL";

/// `Wire::encode` の出力 — transport が送るもの。
pub struct DevinPayload {
    /// `host`（trailing slash 除去済み）。
    pub host: String,
    /// request を組み立てた api key（JWT mint に使う）。
    pub api_key: Secret,
    /// `encodeMessage(1, …)` の中身 — metadata field 群（jwt 無し）。
    pub metadata_body: Vec<u8>,
    /// field 3 prompts 以降の request body。
    pub tail: Vec<u8>,
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// `streamDevin` の request 側（context map + protobuf encode）。
/// transport には触れない — JWT は `DevinTransport` が mint して
/// metadata 末尾へ `encodeString(21, jwt)` として append する
/// （TS `buildMetadata` の `parts.push(encodeString(21, jwt))` と
/// byte 一致）。
#[derive(Clone)]
pub struct DevinWire {
    /// Devin api key（credentials.toml / stored credential 由来）。
    pub api_key: Secret,
    /// Connect endpoint host。
    pub host: String,
    /// Pi-facing model id（`resolveModelUid` の入力）。
    pub model_id: String,
    /// catalog の `thinkingLevelMap`（None なら model_id をそのまま送る）。
    pub thinking_level_map: Option<cheaper_runtime::catalog::schema::ThinkingLevelMap>,
}

impl Wire for DevinWire {
    type Op = Completion;
    type Payload = DevinPayload;
    type Frame = DevinFrame;
    type Decoder<'id> = DevinDecoder<'id>;

    fn describe(&self) -> Descriptor<'_> {
        Descriptor::new(PROVIDER_NAME).model(self.model_id.as_str())
    }

    fn encode(
        &self,
        request: WireRequest<Self>,
        _mode: Mode,
    ) -> Result<Self::Payload, EncodeError> {
        let api_key = self.api_key.expose();
        if api_key.is_empty() {
            return Err(EncodeError::request(
                "No Devin credentials. Run `cheaper auth login devin` (uses the local Devin CLI).",
            ));
        }
        let mapped = map_request_to_chat(&request);
        let ids = session_ids(api_key, &self.host);
        // TS `streamDevin` は `resolveModelUid(model.id, map,
        // options?.reasoning)` で、`options.reasoning` は session が
        // `thinkingLevel: "off"` に pin する（`session.ts` —
        // worker session は常に off）。Rig request には per-request
        // reasoning level が無いため TS の固定値 `"off"` をそのまま
        // 再現する。spec に reasoning field が増えたらここへ流す。
        let model_uid = crate::devin::models::resolve_model_uid(
            &self.model_id,
            self.thinking_level_map.as_ref(),
            Some("off"),
        );
        let prompt_id = uuid::Uuid::new_v4().to_string();
        let trigger_id = uuid::Uuid::new_v4().to_string();
        let parts = build_get_chat_message_request_parts(&ChatRequestArgs {
            api_key,
            model_uid: &model_uid,
            messages: &mapped.messages,
            tools: if mapped.tools.is_empty() {
                None
            } else {
                Some(&mapped.tools)
            },
            cascade_id: &ids.cascade_id,
            prompt_id: &prompt_id,
            session_id: &ids.session_id,
            request_id: now_millis(),
            trigger_id: &trigger_id,
            max_output_tokens: request.max_tokens,
        });
        Ok(DevinPayload {
            host: self.host.clone(),
            api_key: self.api_key.clone(),
            metadata_body: parts.metadata_body,
            tail: parts.tail,
        })
    }

    fn decoder<'id>(&self) -> Self::Decoder<'id> {
        DevinDecoder::new()
    }
}

/// `classify` が生む 1 frame 分の decoded event 列（`Decoder::Event`
/// の関連型なので pub 必須。variant は内部 detail）。
#[derive(Debug)]
pub enum DevinFrameEvent {
    /// proto message 内の event 列。
    Events(Vec<ChatEvent>),
    /// EOS trailer — `Some` は provider 側 error message。
    End(Option<String>),
}

/// `DevinWire` の reply decoder。`TextPart`/`ReasoningPart`/`CallPart`
/// を open 状態で保持し、event 種の遷移で close する（TS の
/// `closeText`/`closeThinking`/`closeTool` に対応）。
pub struct DevinDecoder<'id> {
    text: Option<TextPart<'id>>,
    reasoning: Option<ReasoningPart<'id>>,
    call: Option<CallPart<'id>>,
    finish_reason: Option<ChatFinishReason>,
    usage: Option<ChatUsage>,
}

impl<'id> DevinDecoder<'id> {
    fn new() -> Self {
        Self {
            text: None,
            reasoning: None,
            call: None,
            finish_reason: None,
            usage: None,
        }
    }

    fn close_text(&mut self, out: &mut Out<'id, Completion>) {
        if let Some(part) = self.text.take() {
            out.close_text(part);
        }
    }

    fn close_reasoning(&mut self, out: &mut Out<'id, Completion>) {
        if let Some(part) = self.reasoning.take() {
            out.close_reasoning(part, Seal::default());
        }
    }

    /// open call を close。malformed JSON args は `close_call` の
    /// `MalformedToolInput` error として surface する（TS は
    /// `JSON.parse` 失敗を飲み込んで途中値を残す — そこだけ挙動が
    /// 厳格化される divergence）。
    // `ProviderError` は rig 所有の大きな Err 型 — box 化できないので
    // lint を局所抑制する。
    #[allow(clippy::result_large_err)]
    fn close_call(&mut self, out: &mut Out<'id, Completion>) -> Result<(), ProviderError> {
        if let Some(part) = self.call.take() {
            out.close_call(part)?;
        }
        Ok(())
    }
}

impl<'id> Decoder<'id, Completion, DevinFrame> for DevinDecoder<'id> {
    type Event = DevinFrameEvent;

    fn classify(&self, frame: DevinFrame) -> WireEvent<Self::Event> {
        match frame {
            DevinFrame::EndOfStream { error } => WireEvent::Known(DevinFrameEvent::End(error)),
            DevinFrame::Message(proto) => match decode_chat_frame(&proto) {
                Ok(events) => WireEvent::Known(DevinFrameEvent::Events(events)),
                Err(error) => WireEvent::Corrupt(<serde_json::Error as serde::de::Error>::custom(
                    error.to_string(),
                )),
            },
        }
    }

    fn decode(
        &mut self,
        event: Self::Event,
        mut out: Out<'id, Completion>,
    ) -> Result<Flow, ProviderError> {
        match event {
            DevinFrameEvent::Events(events) => {
                for event in events {
                    match event {
                        ChatEvent::Text(text) => {
                            self.close_reasoning(&mut out);
                            if self.text.is_none() {
                                self.text = Some(out.text());
                            }
                            if let Some(part) = &self.text {
                                out.push_text(part, &text);
                            }
                        }
                        ChatEvent::Reasoning(text) => {
                            self.close_text(&mut out);
                            if self.reasoning.is_none() {
                                self.reasoning = Some(out.reasoning());
                            }
                            if let Some(part) = &self.reasoning {
                                out.push_reasoning(part, &text);
                            }
                        }
                        ChatEvent::ToolCallStart { id, name } => {
                            self.close_text(&mut out);
                            self.close_reasoning(&mut out);
                            self.close_call(&mut out)?;
                            let name = ToolName::try_from(name.as_str()).map_err(|_| {
                                ProviderError::Response(format!(
                                    "devin tool call has empty name (id {id})"
                                ))
                            })?;
                            self.call = Some(out.call(CallId::from_wire(id), name)?);
                        }
                        ChatEvent::ToolCallArgs { delta, .. } => {
                            // open call が無ければ TS 同様 skip。
                            if let Some(part) = &self.call {
                                out.push_arguments(part, &delta);
                            }
                        }
                        ChatEvent::Finish(reason) => {
                            // TS `stream.ts` finish handler は open
                            // content part を全て close してから
                            // stopReason を確定する（EOS まで開いた
                            // ままにしない）。
                            self.close_text(&mut out);
                            self.close_reasoning(&mut out);
                            self.close_call(&mut out)?;
                            self.finish_reason = Some(reason);
                        }
                        ChatEvent::Usage(usage) => {
                            self.usage = Some(usage);
                        }
                    }
                }
                Ok(Flow::More)
            }
            DevinFrameEvent::End(error) => {
                self.close_text(&mut out);
                self.close_reasoning(&mut out);
                self.close_call(&mut out)?;
                if let Some(error) = error {
                    // Connect trailer の `"error"` — TS `throw Error(trailerError)`。
                    return Err(ProviderError::Provider(error));
                }
                let reason = self
                    .finish_reason
                    .map(|reason| match reason {
                        ChatFinishReason::Stop => FinishReason::Stop,
                        ChatFinishReason::ToolCalls => FinishReason::ToolCalls,
                        ChatFinishReason::Length => FinishReason::Length,
                        // Rig 側の normalized vocabulary には
                        // ContentFilter があるため、Pi `stopReason` への
                        // 潰し込み（"stop"）ではなく保持する。
                        ChatFinishReason::ContentFilter => FinishReason::ContentFilter,
                    })
                    // TS `output.stopReason` は初期値 "stop"。
                    .unwrap_or(FinishReason::Stop);
                let usage = self.usage.take().map_or_else(Usage::default, |u| Usage {
                    input_tokens: Some(u.prompt_tokens.unwrap_or(0)),
                    output_tokens: Some(u.completion_tokens.unwrap_or(0)),
                    total_tokens: Some(u.total_tokens.unwrap_or(
                        u.prompt_tokens.unwrap_or(0) + u.completion_tokens.unwrap_or(0),
                    )),
                    cached_input_tokens: u.cached_input_tokens,
                    cache_creation_input_tokens: u.cache_creation_input_tokens,
                    tool_use_prompt_tokens: None,
                    reasoning_tokens: None,
                });
                Ok(out.end(Finish {
                    usage,
                    reason: Some(reason),
                    message_id: None,
                    response_id: None,
                    model: None,
                }))
            }
        }
    }

    /// EOS trailer 未到達の stream 終端 — TS `"Devin stream ended
    /// without an EOS trailer"` 相当。
    fn eof(&mut self, _out: Out<'id, Completion>) -> Result<Flow, ProviderError> {
        Err(ProviderError::Truncated)
    }
}

/// `JwtError` → `ProviderError`（HTTP status は保持する）。
fn jwt_error(e: JwtError) -> ProviderError {
    match e {
        JwtError::Transport(error) => ProviderError::from_transport_error(*error),
        JwtError::Status { status, body } => ProviderError::from_http_response(status, body),
        JwtError::NoJwt => ProviderError::Response("GetUserJwt returned no JWT".to_string()),
        JwtError::Timeout => {
            ProviderError::from_transport_error(rig::http_client::Error::instance(
                std::io::Error::new(std::io::ErrorKind::TimedOut, "GetUserJwt timed out"),
            ))
        }
        JwtError::Request(message) => ProviderError::request(std::io::Error::other(message)),
    }
}

/// Connect deframe 由来の error を `ProviderError` へ。
fn deframe_error(e: DeframeError<rig::http_client::Error>) -> ProviderError {
    match e {
        DeframeError::Chunk(error) => ProviderError::from_transport_error(error),
        DeframeError::Decode(error) => {
            ProviderError::Response(format!("Devin stream decode failed: {error}"))
        }
    }
}

/// TS `GetChatMessage HTTP {resp.status}: {body.slice(0, 300)}` — 非 2xx
/// 応答の message。`status` は数値のみ（TS `resp.status`）。body の
/// 切り詰めは char 単位（jwt.rs の 240 文字切り詰めと同じ近似）。
pub fn chat_status_message(status: StatusCode, body: &str) -> String {
    let truncated: String = body.chars().take(300).collect();
    format!("GetChatMessage HTTP {}: {truncated}", status.as_u16())
}

/// `send_streaming` の error を `ProviderError` へ。非 2xx 応答
/// （`InvalidStatusCodeWithDetails`）は TS message 形式へ写し、status /
/// headers は `ProviderResponseError` に保持する（adapter の transient
/// 分類 — 429/5xx retry — が status を参照するため）。
pub fn chat_send_error(error: http_client::Error) -> ProviderError {
    match error {
        http_client::Error::InvalidStatusCodeWithDetails {
            status,
            body,
            headers,
        } => ProviderError::ProviderResponse(
            ProviderResponseError::new(status, chat_status_message(status, &body))
                .with_headers(Some(headers)),
        ),
        other => ProviderError::from_transport_error(other),
    }
}

/// `GetChatMessage` の HTTP transport。`send` 内で JWT を
/// cache 経由で mint → Connect frame POST → `deframe_connect`。
#[derive(Clone)]
pub struct DevinTransport {
    http: DynHttpClient,
}

impl DevinTransport {
    pub fn new(http: DynHttpClient) -> Self {
        Self { http }
    }
}

impl Transport<DevinWire> for DevinTransport {
    fn send(&self, payload: DevinPayload, _exchange: Exchange) -> Opening<DevinFrame> {
        let http = self.http.clone();
        Opening::new(async move {
            let jwt = jwt::get_cached_user_jwt(&http, payload.api_key.expose(), &payload.host)
                .await
                .map_err(jwt_error)?;

            // `encodeMessage(1, metadata_body + encodeString(21, jwt)) + tail`
            // — TS `buildGetChatMessageRequest` と byte 同一。
            let mut metadata_body = payload.metadata_body;
            metadata_body.extend_from_slice(&crate::devin::wire::encode_string(21, &jwt));
            let mut body = crate::devin::wire::encode_message(1, &metadata_body);
            body.extend_from_slice(&payload.tail);
            let framed = crate::devin::wire::frame_connect_stream(&body, true);

            let request = Request::post(format!("{}{}", payload.host, CHAT_ENDPOINT))
                .header("content-type", "application/connect+proto")
                .header("connect-protocol-version", "1")
                .header("connect-content-encoding", "gzip")
                .header("connect-accept-encoding", "gzip")
                .body(Bytes::from(framed))
                .map_err(ProviderError::request)?;
            let response = http
                .send_streaming(request)
                .await
                .map_err(chat_send_error)?;
            let status = response.status();
            if !status.is_success() {
                // ReqwestClient 系は非 2xx を Err 化するため通常ここには
                // 来ないが、status を error 化しない `HttpClientExt` でも
                // TS `!resp.ok` 経路を再現する（body を drain して
                // message に載せる）。
                let mut body = response.into_body();
                let mut bytes = Vec::new();
                while let Some(chunk) = futures::StreamExt::next(&mut body).await {
                    bytes.extend_from_slice(&chunk.map_err(ProviderError::from_transport_error)?);
                }
                return Err(ProviderError::ProviderResponse(ProviderResponseError::new(
                    status,
                    chat_status_message(status, &String::from_utf8_lossy(&bytes)),
                )));
            }
            let headers = response.headers().clone();
            // `Opened` は `Result<_, ProviderError>` item を要求するため
            // Err 型は rig 所有のまま（lint 局所抑制）。
            #[allow(clippy::result_large_err)]
            let frames = futures::StreamExt::map(deframe_connect(response.into_body()), |item| {
                item.map_err(deframe_error)
            });
            Ok(Opened::new(frames).with_http(status, headers))
        })
    }
}

/// TS `streamDevin` の host 解決 — `options.env.DEVIN_API_SERVER_URL`
/// → `auth.baseUrl`（credentials.toml の `api_server_url`）→ protocol
/// 既定 host。trailing slash は TS `.replace(/\/$/,"")` 同様 1 個だけ
/// 除去する（`trim_end_matches('/')` は全ての slash を削る）。
pub fn resolve_host(env: &dyn ProviderEnv, auth: Option<&ResolvedAuth>) -> String {
    let host = env
        .env(HOST_ENV)
        .or_else(|| auth.and_then(|auth| auth.base_url.clone()))
        .unwrap_or_else(|| DEFAULT_HOST.to_string());
    host.strip_suffix('/').unwrap_or(&host).to_string()
}

/// `SessionSpec`（resolved catalog model + auth）→ adapter 用
/// `CompletionModel`。runtime の `RigSessionFactory` が
/// `CompletionModelFactory` として呼ぶ。
#[derive(Clone)]
pub struct DevinCompletionModelFactory {
    http: DynHttpClient,
    /// env 解決 seam（`DEVIN_API_SERVER_URL`）。`std::env` 直接参照
    /// ではなく `ProviderEnv` 経由 — test は MapEnv 相当を注入する。
    env: Arc<dyn ProviderEnv>,
}

impl DevinCompletionModelFactory {
    pub fn new(http: DynHttpClient) -> Self {
        Self {
            http,
            env: Arc::new(ProcessEnv),
        }
    }
    /// scoped env（test / composition root の overlay）を注入する seam。
    pub fn with_env(http: DynHttpClient, env: Arc<dyn ProviderEnv>) -> Self {
        Self { http, env }
    }
}

impl Default for DevinCompletionModelFactory {
    /// 既定の process-wide reqwest client。
    fn default() -> Self {
        Self::new(DynHttpClient::new(
            rig::rig_reqwest::ReqwestClient::default(),
        ))
    }
}

impl CompletionModelFactory for DevinCompletionModelFactory {
    fn completion_model(&self, spec: &SessionSpec<'_>) -> Result<CompletionModel, CheaperError> {
        if spec.model.api.as_str() != API_KIND {
            return Err(CheaperError::new(
                ErrorCode::InvalidModelConfig,
                format!(
                    "devin provider received non-{API_KIND} model api `{}`",
                    spec.model.api
                ),
            ));
        }
        let api_key = spec
            .auth
            .and_then(|auth| auth.api_key.clone())
            .ok_or_else(|| {
                CheaperError::new(
                    ErrorCode::ProviderFailure,
                    "No Devin credentials. Run `cheaper auth login devin` (uses the local Devin CLI).",
                )
            })?;
        // `spec.output_schema`（repair session の json_schema hint）は
        // 意図的に wire へ載せない — TS `streamDevin` は
        // `model.samplingParams.response_format` を一切読まず、
        // repair でも request へ encode しない（corrective prompt +
        // attempt 上限のみが contract）。TS と同じく無視する。
        let _ = spec.output_schema;
        // TS `streamDevin`: `DEVIN_API_SERVER_URL` env → credential の
        // `api_server_url`（`auth.baseUrl` 経由）→ protocol 既定 host。
        let host = resolve_host(self.env.as_ref(), spec.auth);
        let wire = DevinWire {
            api_key: Secret::from(api_key),
            host,
            model_id: spec.model.id.clone(),
            thinking_level_map: spec.model.thinking_level_map.clone(),
        };
        let transport = DevinTransport::new(self.http.clone());
        Ok(CompletionModel::from_rig(Model::new(wire, transport))
            .with_max_tokens(Some(spec.model.max_tokens as u64)))
    }
}

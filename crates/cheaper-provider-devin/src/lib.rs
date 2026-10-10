//! `cheaper-provider-devin` — Devin Local provider plugin の Rust port
//! （`packages/runtime-pi/src/providers/devin/`）。
//!
//! Vendored upstream: [`pi-devin`](https://github.com/kashyab12/pi-devin)
//! （Copyright (c) 2026 Kashyab Ambarani — MIT License。NOTICE 参照）。
//!
//! 構成:
//!
//! - [`wire`] — protobuf-lite encoder/decoder + Connect 5-byte framing
//! - [`metadata`] — `buildMetadata`（IDE/version/session/request/trigger）
//! - [`jwt`] — `GetUserJwt` unary POST + exp 抽出 + 60s 余裕 cache +
//!   in-flight dedup
//! - [`context_map`] — Rig `CompletionRequest` → Devin chat message/tool
//!   構造
//! - [`stream`] — `buildGetChatMessageRequest` + frame → event decode +
//!   Connect deframer
//! - [`cli`] — Devin CLI child-process adapter（`DEVIN_CLI` override、
//!   isolated env、timeout）
//! - [`credentials`] — isolated `credentials.toml` + auth status/login/
//!   logout
//! - [`models`] — `devin models list` catalog → `ModelDefinitionJson`
//! - [`rig_provider`] — `Wire`/`Transport`/`Decoder` →
//!   `CompletionModel::from_rig`
//! - [`plugin`] — `CheaperProviderPlugin` registration

pub mod cli;
pub mod context_map;
pub mod credentials;
pub mod jwt;
pub mod metadata;
pub mod models;
pub mod plugin;
pub mod rig_provider;
pub mod stream;
pub mod wire;

pub use cli::{
    devin_child_env, CliError, CliRunOptions, DevinCli, DevinCliNotFoundError, DevinCliResult,
    DevinCliRunner, ProcessCliRunner, DEFAULT_TIMEOUT,
};
pub use context_map::{
    map_request_to_chat, ChatContent, ChatHistoryItem, ChatRole, ChatToolCall, ContentPart,
    MappedChat, ToolDef,
};
pub use credentials::{
    devin_auth_status, devin_credentials_path, devin_data_dir, login_with_devin_cli,
    logout_devin_session, read_credentials_at, read_devin_credentials,
    resolve_global_credentials_path, DevinAuthStatus, DevinCredentials,
};
pub use jwt::{clear_cached_user_jwt, get_cached_user_jwt, mint_user_jwt, JwtError, MintedUserJwt};
pub use metadata::{build_metadata, client_version, MetadataInput, CLIENT_IDE};
pub use models::{
    fallback_models, load_cli_catalog, models_from_catalog, resolve_model_uid, DevinCatalog,
    DevinFamily, DevinVariant, THINKING_ORDER,
};
pub use plugin::{DevinProviderPlugin, PROVIDER_ID};
pub use rig_provider::{
    DevinCompletionModelFactory, DevinDecoder, DevinPayload, DevinTransport, DevinWire, API_KIND,
    DEFAULT_HOST, HOST_ENV,
};
pub use stream::{
    build_get_chat_message_request_parts, collapse_system_into_user, decode_chat_frame,
    deframe_connect, session_ids, trailer_error, ChatEvent, ChatFinishReason, ChatRequestArgs,
    ChatUsage, DeframeError, DevinFrame, RequestParts, SessionIds, CHAT_ENDPOINT,
};
pub use wire::{
    decode_varint, encode_fixed64_field, encode_message, encode_string, encode_tag,
    encode_timestamp_body, encode_varint, encode_varint_field, frame_connect_stream, gunzip, gzip,
    iter_fields, DecodeError, FieldIter, FieldValue, ProtoField,
};

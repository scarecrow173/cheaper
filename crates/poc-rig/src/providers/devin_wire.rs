//! Devin provider skeleton: a custom `Wire`/`Decoder`/`Transport` triple.
//!
//! T4 scope is trait-surface feasibility only — Connect framing, gzip, JWT,
//! metadata, and the context map must be representable here; no traffic is
//! sent. `send` fails every opening with a provider error.

use rig::completion::CompletionRequest;
use rig::driver::{Exchange, Opening, Transport};
use rig::error::{EncodeError, ProviderError};
use rig::operation::Completion;
use rig::streaming::UnknownPayload;
use rig::wire::{
    Capabilities, Decoder, Descriptor, Flow, Mode, Out, Secret, Wire, WireEvent,
};

/// Devin endpoint descriptor: Connect-protocol chat completion target.
/// Real fields to be proven in Phase 3: base URL, JWT bearer, Connect
/// content-type (`application/connect+json` / `+proto`), gzip flag, and the
/// Devin-specific metadata/context-map envelope around each request.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DevinCompletionWire {
    /// e.g. `https://api.devin.ai/...`
    pub base_url: String,
    /// JWT credential; never serialized by `Secret`.
    pub api_key: Secret,
    /// Model identifier.
    pub model: String,
    /// Devin metadata/context-map entries attached to the Connect envelope.
    pub metadata: std::collections::BTreeMap<String, String>,
}

/// The Connect request payload: an HTTP request carrying the encoded
/// envelope (framed JSON + gzip when negotiated).
pub type DevinPayload = http::Request<Vec<u8>>;

/// One Connect reply frame: a length-prefixed envelope chunk.
#[derive(Debug, Clone)]
pub struct DevinFrame(pub Vec<u8>);

/// Incremental decoder: classifies Connect envelope chunks and writes known
/// events into the reply's writer.
#[derive(Default)]
pub struct DevinDecoder<'id> {
    _id: std::marker::PhantomData<&'id ()>,
}

/// Typed decode event for Devin replies (assistant text, tool call, usage,
/// terminal metadata).
#[derive(Debug)]
pub enum DevinEvent {
    TextDelta(String),
    Final,
}

impl<'id> Decoder<'id, Completion, DevinFrame> for DevinDecoder<'id> {
    type Event = DevinEvent;

    fn classify(&self, _frame: DevinFrame) -> WireEvent<Self::Event> {
        // Connect envelope parsing lands here in Phase 3.
        WireEvent::Unknown {
            event_type: "devin.unparsed".into(),
            value: UnknownPayload::new(serde_json::Value::Null),
        }
    }

    fn decode(
        &mut self,
        _event: Self::Event,
        _out: Out<'id, Completion>,
    ) -> Result<Flow, ProviderError> {
        Ok(Flow::More)
    }
}

impl Wire for DevinCompletionWire {
    type Op = Completion;
    type Payload = DevinPayload;
    type Frame = DevinFrame;
    type Decoder<'id> = DevinDecoder<'id>;

    fn describe(&self) -> Descriptor<'_> {
        Descriptor::new("devin")
            .model(self.model.as_str())
            .capabilities(Capabilities::default())
    }

    fn encode(&self, _request: CompletionRequest, _mode: Mode) -> Result<Self::Payload, EncodeError> {
        // Feasibility proof: the request serializes into a Connect envelope
        // carrying metadata. The actual schema lands in Phase 3.
        let body = serde_json::to_vec(&serde_json::json!({
            "metadata": self.metadata,
            "request": "<encoded chat request>",
        }))
        .map_err(EncodeError::from)?;
        http::Request::builder()
            .method(http::Method::POST)
            .uri(format!("{}/chat", self.base_url))
            .header(http::header::CONTENT_TYPE, "application/connect+json")
            .header(http::header::CONTENT_ENCODING, "gzip")
            .header(
                http::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key.expose()),
            )
            .body(body)
            .map_err(EncodeError::from)
    }

    fn decoder<'id>(&self) -> Self::Decoder<'id> {
        DevinDecoder::default()
    }
}

/// Devin transport: an HTTP client in production; the skeleton fails every
/// send to prove the seam compiles and separates concerns.
#[derive(Debug, Clone, Default)]
pub struct DevinTransport;

impl Transport<DevinCompletionWire> for DevinTransport {
    fn send(&self, _payload: DevinPayload, _exchange: Exchange) -> Opening<DevinFrame> {
        Opening::failed(ProviderError::Provider(
            "devin transport skeleton: no transport bound".into(),
        ))
    }
}

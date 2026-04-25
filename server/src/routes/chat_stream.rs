//! Streaming chat completion API handler using Server-Sent Events (SSE).
//!
//! `POST /api/chat/completions/stream` — pipes the LLM provider's `mpsc`
//! stream directly to the client as SSE events. Each `ChatCompletionChunk`
//! is serialised to JSON and wrapped in a `data:` event. The stream
//! terminates with `data: [DONE]`. Errors encountered mid-stream are
//! reported as `event: error` SSE messages before closing the connection.

use crate::providers::{ChatCompletionChunk, ChatCompletionRequest, ProviderError};
use crate::routes::error::{error_response, map_provider_error};
use crate::state::AppState;
use axum::extract::{rejection::JsonRejection, Json, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use futures_util::stream;
use futures_util::StreamExt;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Internal state machine for the SSE stream produced by the handler.
enum SseStreamState {
    /// Actively receiving chunks from the provider channel.
    Receiving(mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>),
    /// Stream is finished — no more events to emit.
    Done,
}

/// `POST /api/chat/completions/stream` — streams chat completion chunks
/// to the client using Server-Sent Events.
///
/// Returns an SSE response with `Content-Type: text/event-stream`.
/// Each chunk is serialised as `data: {json}`. When the provider
/// channel closes cleanly, a final `data: [DONE]` event is sent.
/// If an error occurs mid-stream, an `event: error` message is sent
/// and the stream terminates.
pub async fn chat_completion_stream(
    State(state): State<AppState>,
    payload: Result<Json<ChatCompletionRequest>, JsonRejection>,
) -> impl IntoResponse {
    let request = match payload {
        Ok(Json(request)) => request,
        Err(err) => {
            warn!(error = %err, "failed to parse streaming chat completion request");
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("Failed to parse JSON request: {err}"),
            );
        }
    };

    info!(
        model = %request.model,
        message_count = request.messages.len(),
        "forwarding streaming chat completion request"
    );

    let receiver = match state.provider.chat_completion_stream(request).await {
        Ok(rx) => rx,
        Err(err) => {
            let (status, message) = map_provider_error(&err);
            error!(status = %status, error = %err, "streaming chat completion failed to start");
            return error_response(status, message);
        }
    };

    let sse_stream = build_sse_stream(receiver);
    let sse = Sse::new(sse_stream).keep_alive(KeepAlive::default());

    sse.into_response()
}

/// Build the SSE event stream from the provider's mpsc receiver.
///
/// Uses [`futures_util::stream::unfold`] with a state machine to:
/// 1. Yield `data: {json}` for each successful chunk.
/// 2. Yield `event: error` + `data: {message}` on provider errors, then stop.
/// 3. Yield `data: [DONE]` when the channel closes cleanly, then stop.
fn build_sse_stream(
    receiver: mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>,
) -> stream::BoxStream<'static, Result<Event, Infallible>> {
    stream::unfold(SseStreamState::Receiving(receiver), |state| async move {
        match state {
            SseStreamState::Receiving(mut rx) => match rx.recv().await {
                Some(Ok(chunk)) => {
                    let json = serde_json::to_string(&chunk).unwrap_or_else(|_| "{}".to_string());
                    let event = Event::default().data(json);
                    Some((Ok(event), SseStreamState::Receiving(rx)))
                }
                Some(Err(err)) => {
                    let event = Event::default().event("error").data(err.to_string());
                    Some((Ok(event), SseStreamState::Done))
                }
                None => {
                    let event = Event::default().data("[DONE]");
                    Some((Ok(event), SseStreamState::Done))
                }
            },
            SseStreamState::Done => None,
        }
    })
    .boxed()
}

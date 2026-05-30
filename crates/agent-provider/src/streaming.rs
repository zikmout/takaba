use anyhow::Result;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Response;

use crate::types::ChatChunk;

/// Parse an NDJSON streaming response into a stream of ChatChunk.
pub fn parse_ndjson_stream(response: Response) -> BoxStream<'static, Result<ChatChunk>> {
    let stream = async_stream(response);
    Box::pin(stream)
}

fn async_stream(response: Response) -> impl futures::Stream<Item = Result<ChatChunk>> {
    async_stream::stream! {
        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = bytes_stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                match serde_json::from_str::<ChatChunk>(&line) {
                    Ok(chat_chunk) => yield Ok(chat_chunk),
                    Err(e) => {
                        tracing::warn!("failed to parse NDJSON line: {e}: {line}");
                    }
                }
            }
        }

        // Process any remaining data in buffer
        let remaining = buffer.trim().to_string();
        if !remaining.is_empty() {
            match serde_json::from_str::<ChatChunk>(&remaining) {
                Ok(chat_chunk) => yield Ok(chat_chunk),
                Err(e) => {
                    tracing::warn!("failed to parse final NDJSON: {e}: {remaining}");
                }
            }
        }
    }
}

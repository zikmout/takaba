use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::types::{ChatChunk, ChatRequest, ChatResponse, ProviderMetadata};

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> anyhow::Result<ChatResponse>;

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatChunk>>>;

    fn metadata(&self) -> ProviderMetadata;
}

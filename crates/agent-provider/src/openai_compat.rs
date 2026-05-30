use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::provider::LlmProvider;
use crate::types::{ChatChunk, ChatRequest, ChatResponse, ProviderMetadata};

pub struct OpenAICompatibleProvider {
    base_url: String,
    model: String,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl OpenAICompatibleProvider {
    pub fn new(base_url: String, model: String, api_key: Option<String>) -> Self {
        Self {
            base_url,
            model,
            api_key,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAICompatibleProvider {
    async fn chat(&self, _request: ChatRequest) -> anyhow::Result<ChatResponse> {
        todo!("OpenAI-compatible provider not yet implemented")
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatChunk>>> {
        todo!("OpenAI-compatible provider streaming not yet implemented")
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "openai_compatible".to_string(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
        }
    }
}

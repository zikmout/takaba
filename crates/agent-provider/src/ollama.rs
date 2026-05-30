use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use std::time::Duration;

use crate::provider::LlmProvider;
use crate::streaming::parse_ndjson_stream;
use crate::types::{ChatChunk, ChatRequest, ChatResponse, ProviderMetadata};

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
    timeout: Duration,
}

impl OllamaProvider {
    pub fn new(base_url: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            base_url,
            model,
            timeout: Duration::from_secs(300),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, mut request: ChatRequest) -> anyhow::Result<ChatResponse> {
        request.model = self.model.clone();

        let payload = build_ollama_payload(&request, false);

        let response = self
            .client
            .post(self.chat_url())
            .timeout(self.timeout)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error {status}: {body}");
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response)
    }

    async fn chat_stream(
        &self,
        mut request: ChatRequest,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatChunk>>> {
        request.model = self.model.clone();

        let payload = build_ollama_payload(&request, true);

        let response = self
            .client
            .post(self.chat_url())
            .timeout(self.timeout)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error {status}: {body}");
        }

        Ok(parse_ndjson_stream(response))
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "ollama".to_string(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
        }
    }
}

fn build_ollama_payload(request: &ChatRequest, stream: bool) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "model": request.model,
        "messages": request.messages,
        "stream": stream,
    });

    if let Some(tools) = &request.tools {
        payload["tools"] = serde_json::to_value(tools).unwrap_or_default();
    }

    if let Some(options) = &request.options {
        payload["options"] = serde_json::to_value(options).unwrap_or_default();
    }

    payload
}

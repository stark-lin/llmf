use std::io::Read;
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};

use crate::workspace::Config;
use crate::{EXIT_API, LlmfError, Result, check_interrupted};

const MAX_RESPONSE_BODY_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiResponse {
    pub content: String,
}

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    api_url: String,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct RequestBody<'a> {
    model: &'a str,
    messages: &'a [Message],
    stream: bool,
}

#[derive(Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoice>,
}

#[derive(Deserialize)]
struct CompletionChoice {
    message: CompletionMessage,
}

#[derive(Deserialize)]
struct CompletionMessage {
    content: Option<String>,
}

impl ApiClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| {
                LlmfError::new(EXIT_API, format!("cannot create HTTP client: {error}"))
            })?;
        Ok(Self {
            client,
            api_url: config.api_url.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
        })
    }

    pub fn complete(&self, messages: &[Message]) -> Result<ApiResponse> {
        check_interrupted()?;
        let body = RequestBody {
            model: &self.model,
            messages,
            stream: false,
        };
        let mut request = self.client.post(&self.api_url).json(&body);
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }
        let response = request.send().map_err(map_request_error)?;
        check_interrupted()?;
        parse_response(response)
    }
}

fn map_request_error(error: reqwest::Error) -> LlmfError {
    if let Err(interrupted) = check_interrupted() {
        return interrupted;
    }
    if error.is_timeout() {
        LlmfError::new(EXIT_API, "API request timed out")
    } else {
        // reqwest errors can embed a request URL. Do not surface it because a
        // misconfigured URL can itself contain sensitive material.
        LlmfError::new(EXIT_API, "API request failed")
    }
}

fn parse_response(response: Response) -> Result<ApiResponse> {
    let status = response.status();
    if !status.is_success() {
        return Err(LlmfError::new(
            EXIT_API,
            format!("API returned HTTP {}", status.as_u16()),
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BODY_BYTES)
    {
        return Err(LlmfError::new(EXIT_API, "API response body too large"));
    }

    let mut bytes = Vec::new();
    response
        .take(MAX_RESPONSE_BODY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| match check_interrupted() {
            Err(error) => error,
            Ok(()) => LlmfError::new(EXIT_API, "cannot read API response"),
        })?;
    check_interrupted()?;
    if bytes.len() as u64 > MAX_RESPONSE_BODY_BYTES {
        return Err(LlmfError::new(EXIT_API, "API response body too large"));
    }
    let value: CompletionResponse = serde_json::from_slice(&bytes)
        .map_err(|_| LlmfError::new(EXIT_API, "API returned invalid JSON"))?;
    let content = value
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or_else(|| {
            LlmfError::new(EXIT_API, "API response missing choices[0].message.content")
        })?;
    Ok(ApiResponse {
        content: content.to_owned(),
    })
}

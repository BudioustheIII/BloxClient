//! HTTP client for Roblox's public web APIs. No cookies or credentials are
//! sent. Callers only build URLs from the fixed hosts in `api.rs`.

use std::time::Duration;

use reqwest::header::{CONTENT_TYPE, RETRY_AFTER};
use reqwest::{Client, RequestBuilder, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::error::RobloxError;

const MAX_BODY_BYTES: usize = 6 * 1024 * 1024;
const MAX_ATTEMPTS: usize = 3;
const DEFAULT_RETRY: Duration = Duration::from_millis(700);
const MAX_RETRY: Duration = Duration::from_secs(3);

pub struct RobloxHttp {
    client: Client,
    /// Random per app run, required by the explore and search APIs.
    session_id: String,
}

fn network(error: impl std::fmt::Display) -> RobloxError {
    RobloxError::Network(error.to_string())
}

impl RobloxHttp {
    pub fn new() -> Result<Self, RobloxError> {
        let client = Client::builder()
            .user_agent(concat!("BloxClient/", env!("CARGO_PKG_VERSION")))
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(network)?;
        Ok(Self {
            client,
            session_id: uuid::Uuid::new_v4().to_string(),
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: Url) -> Result<T, RobloxError> {
        self.send_json(url.path(), || self.client.get(url.clone())).await
    }

    /// For public lookup endpoints that take their ids in a JSON body.
    pub async fn post_json<B: Serialize, T: DeserializeOwned>(&self, url: Url, body: &B) -> Result<T, RobloxError> {
        let payload = serde_json::to_vec(body).map_err(|error| RobloxError::UnexpectedResponse(error.to_string()))?;
        self.send_json(url.path(), || {
            self.client
                .post(url.clone())
                .header(CONTENT_TYPE, "application/json")
                .body(payload.clone())
        })
        .await
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        path: &str,
        request: impl Fn() -> RequestBuilder,
    ) -> Result<T, RobloxError> {
        let mut last_error = RobloxError::Network("no attempts".into());
        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(DEFAULT_RETRY).await;
            }
            match self.try_send(request()).await {
                Ok(body) => {
                    return serde_json::from_slice(&body)
                        .map_err(|error| RobloxError::UnexpectedResponse(format!("{path}: {error}")));
                }
                Err((error, retry_after)) => {
                    let retryable = matches!(error, RobloxError::Network(_) | RobloxError::RateLimited);
                    last_error = error;
                    if !retryable {
                        break;
                    }
                    if let Some(wait) = retry_after {
                        tokio::time::sleep(wait.min(MAX_RETRY)).await;
                    }
                }
            }
        }
        Err(last_error)
    }

    async fn try_send(&self, request: RequestBuilder) -> Result<Vec<u8>, (RobloxError, Option<Duration>)> {
        let mut response = request.send().await.map_err(|error| (network(error), None))?;
        let status = response.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            let wait = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse().ok())
                .map(Duration::from_secs);
            return Err((RobloxError::RateLimited, wait));
        }
        if status == StatusCode::NOT_FOUND || status == StatusCode::BAD_REQUEST {
            return Err((RobloxError::NotFound, None));
        }
        if status.is_server_error() {
            return Err((network(format!("HTTP {status}")), None));
        }
        if !status.is_success() {
            return Err((RobloxError::UnexpectedResponse(format!("HTTP {status}")), None));
        }

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| (network(error), None))? {
            if body.len() + chunk.len() > MAX_BODY_BYTES {
                return Err((RobloxError::UnexpectedResponse("response too large".into()), None));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

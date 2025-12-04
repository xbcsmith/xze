use crate::ai::providers::config::RetryConfig;
use crate::ai::providers::errors::ProviderError;
use crate::ai::providers::retry::RetryStrategy;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

/// Authentication method for API requests
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// Bearer token authentication
    Bearer(String),
    /// API key authentication (header name, key value)
    ApiKey { key: String, header: String },
    /// No authentication
    None,
}

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;

/// Generic API client with retry logic
pub struct ApiClient {
    client: Client,
    retry_strategy: Option<RetryStrategy>,
    headers: HeaderMap,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(timeout: u64, retry_config: Option<RetryConfig>) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|e| ProviderError::Config(format!("Failed to build HTTP client: {}", e)))?;

        let retry_strategy = retry_config.map(RetryStrategy::new);

        Ok(Self {
            client,
            retry_strategy,
            headers: HeaderMap::new(),
        })
    }

    /// Add a default header to all requests
    pub fn with_header(mut self, key: &str, value: &str) -> Result<Self, ProviderError> {
        let name = HeaderName::from_str(key)
            .map_err(|e| ProviderError::Config(format!("Invalid header name: {}", e)))?;
        let val = HeaderValue::from_str(value)
            .map_err(|e| ProviderError::Config(format!("Invalid header value: {}", e)))?;
        self.headers.insert(name, val);
        Ok(self)
    }

    /// Perform a POST request
    pub async fn post<T, R>(
        &self,
        url: &str,
        body: &T,
        auth: &AuthMethod,
    ) -> Result<R, ProviderError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        self.execute_with_retry(move |client| {
            let mut req = client.post(url).json(body);
            req = Self::apply_auth(req, auth);
            req
        })
        .await
    }

    /// Perform a GET request
    pub async fn get<R>(&self, url: &str, auth: &AuthMethod) -> Result<R, ProviderError>
    where
        R: DeserializeOwned,
    {
        self.execute_with_retry(move |client| {
            let mut req = client.get(url);
            req = Self::apply_auth(req, auth);
            req
        })
        .await
    }

    fn apply_auth(req: RequestBuilder, auth: &AuthMethod) -> RequestBuilder {
        match auth {
            AuthMethod::Bearer(token) => req.bearer_auth(token),
            AuthMethod::ApiKey { key, header } => req.header(header, key),
            AuthMethod::None => req,
        }
    }

    async fn execute_with_retry<F, R>(&self, request_builder: F) -> Result<R, ProviderError>
    where
        F: Fn(&Client) -> RequestBuilder,
        R: DeserializeOwned,
    {
        let mut attempt = 1;

        loop {
            let mut request = request_builder(&self.client);

            // Apply default headers
            if !self.headers.is_empty() {
                request = request.headers(self.headers.clone());
            }

            let result = request.send().await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        return response
                            .json::<R>()
                            .await
                            .map_err(|e| ProviderError::Network { source: e });
                    }

                    if let Some(strategy) = &self.retry_strategy {
                        if strategy.should_retry(status, attempt) {
                            let delay = strategy.calculate_delay(attempt, Some(&response));
                            tracing::warn!(
                                "Request failed with status {}, retrying in {:?} (attempt {})",
                                status,
                                delay,
                                attempt
                            );
                            tokio::time::sleep(delay).await;
                            attempt += 1;
                            continue;
                        }
                    }

                    // If we get here, it's a non-retryable error or we ran out of retries
                    let error_text = response.text().await.unwrap_or_default();

                    if status.is_server_error() {
                        return Err(ProviderError::ServerError {
                            status: status.as_u16(),
                        });
                    } else if status == StatusCode::TOO_MANY_REQUESTS {
                        return Err(ProviderError::RateLimit {
                            retry_after: Duration::from_secs(0),
                        });
                    } else {
                        return Err(ProviderError::RequestError {
                            message: format!("Status {}: {}", status, error_text),
                        });
                    }
                }
                Err(e) => {
                    if let Some(strategy) = &self.retry_strategy {
                        // Retry network errors if we have attempts left
                        if attempt < strategy.max_attempts() {
                            let delay = strategy.calculate_delay(attempt, None);
                            tracing::warn!(
                                "Network request failed: {}, retrying in {:?} (attempt {})",
                                e,
                                delay,
                                attempt
                            );
                            tokio::time::sleep(delay).await;
                            attempt += 1;
                            continue;
                        }
                    }
                    return Err(ProviderError::Network { source: e });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_api_client_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result": "success"}"#)
            .create_async()
            .await;

        let client = ApiClient::new(10, None).unwrap();
        let result: serde_json::Value = client
            .post(
                &format!("{}/test", server.url()),
                &serde_json::json!({"data": "test"}),
                &AuthMethod::None,
            )
            .await
            .unwrap();

        assert_eq!(result["result"], "success");
        mock.assert_async().await;
    }
}

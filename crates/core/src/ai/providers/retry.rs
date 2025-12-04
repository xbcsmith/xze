use crate::ai::providers::config::RetryConfig;
use rand::Rng;
use reqwest::{Response, StatusCode};
use std::time::Duration;

/// Strategy for retrying failed requests
pub struct RetryStrategy {
    config: RetryConfig,
}

impl RetryStrategy {
    /// Create a new retry strategy
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Get maximum number of attempts
    pub fn max_attempts(&self) -> u32 {
        self.config.max_attempts
    }

    /// Check if a request should be retried based on status code and attempt count
    pub fn should_retry(&self, status: StatusCode, attempt: u32) -> bool {
        if attempt >= self.config.max_attempts {
            return false;
        }

        match status {
            StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT => true,
            _ => false,
        }
    }

    /// Calculate the delay before the next retry attempt
    pub fn calculate_delay(&self, attempt: u32, response: Option<&Response>) -> Duration {
        // Check Retry-After header first
        if let Some(resp) = response {
            if let Some(retry_after) = resp.headers().get("retry-after") {
                if let Ok(val) = retry_after.to_str() {
                    // Try parsing as seconds
                    if let Ok(secs) = val.parse::<u64>() {
                        return Duration::from_secs(secs);
                    }
                    // TODO: Handle HTTP date format if needed
                }
            }
        }

        // Exponential backoff
        let attempt_idx = attempt.saturating_sub(1); // 0-based index for calculation
        let base_delay = self.config.initial_delay_ms as f64
            * self.config.backoff_multiplier.powi(attempt_idx as i32);
        let max_delay = self.config.max_delay_ms as f64;
        let delay = base_delay.min(max_delay);

        // Add jitter (±10%) to prevent thundering herd
        let jitter = rand::rng().random_range(0.9..1.1);
        Duration::from_millis((delay * jitter) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_retry() {
        let config = RetryConfig {
            max_attempts: 3,
            ..Default::default()
        };
        let strategy = RetryStrategy::new(config);

        assert!(strategy.should_retry(StatusCode::TOO_MANY_REQUESTS, 1));
        assert!(strategy.should_retry(StatusCode::INTERNAL_SERVER_ERROR, 2));
        assert!(!strategy.should_retry(StatusCode::OK, 1));
        assert!(!strategy.should_retry(StatusCode::BAD_REQUEST, 1));
        assert!(!strategy.should_retry(StatusCode::INTERNAL_SERVER_ERROR, 3));
    }

    #[test]
    fn test_calculate_delay_backoff() {
        let config = RetryConfig {
            initial_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_delay_ms: 1000,
            ..Default::default()
        };
        let strategy = RetryStrategy::new(config);

        let delay1 = strategy.calculate_delay(1, None);
        let delay2 = strategy.calculate_delay(2, None);
        let delay3 = strategy.calculate_delay(3, None);

        // Allow for jitter
        assert!(delay1.as_millis() >= 90 && delay1.as_millis() <= 110);
        assert!(delay2.as_millis() >= 180 && delay2.as_millis() <= 220);
        assert!(delay3.as_millis() >= 360 && delay3.as_millis() <= 440);
    }
}

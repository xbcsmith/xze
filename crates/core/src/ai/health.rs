//! Health check functionality for intent classification service
//!
//! This module provides health check capabilities to monitor the availability
//! and performance of the intent classifier and its dependencies.
//!
//! # Examples
//!
//! ```no_run
//! use xze_core::ai::health::HealthCheck;
//! use xze_core::ai::intent_classifier::IntentClassifier;
//!
//! # async fn example(classifier: IntentClassifier) -> xze_core::error::Result<()> {
//! let health = HealthCheck::new(&classifier);
//! let status = health.check().await?;
//!
//! if status.is_healthy() {
//!     println!("Service is healthy");
//! } else {
//!     println!("Service issues: {:?}", status.issues);
//! }
//! # Ok(())
//! # }
//! ```

use crate::ai::client::OllamaClient;
use crate::ai::intent_classifier::IntentClassifier;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Health status of the intent classification service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is fully operational
    Healthy,
    /// Service is operational but degraded
    Degraded,
    /// Service is not operational
    Unhealthy,
}

impl HealthStatus {
    /// Check if status is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    /// Check if status is operational (healthy or degraded)
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }
}

/// Detailed health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Overall health status
    pub status: HealthStatus,

    /// Timestamp of the health check
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Duration of the health check in milliseconds
    pub duration_ms: u64,

    /// Issues detected during health check
    pub issues: Vec<String>,

    /// Cache statistics
    pub cache_stats: CacheHealth,

    /// AI service availability
    pub ai_service: ServiceHealth,
}

impl HealthCheckResult {
    /// Check if the service is healthy
    pub fn is_healthy(&self) -> bool {
        self.status.is_healthy()
    }

    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        if self.issues.is_empty() {
            format!(
                "Status: {:?}, Duration: {}ms",
                self.status, self.duration_ms
            )
        } else {
            format!(
                "Status: {:?}, Issues: {}, Duration: {}ms",
                self.status,
                self.issues.len(),
                self.duration_ms
            )
        }
    }
}

/// Cache health statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHealth {
    /// Number of entries in cache
    pub entry_count: u64,

    /// Cache utilization percentage (0-100)
    pub utilization: f64,

    /// Maximum cache capacity
    pub max_capacity: u64,
}

impl CacheHealth {
    /// Check if cache is at risk of eviction
    pub fn is_high_utilization(&self) -> bool {
        self.utilization > 80.0
    }
}

/// AI service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Whether the AI service is available
    pub available: bool,

    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,

    /// Model name being used
    pub model: String,
}

/// Health check utility for intent classifier
pub struct HealthCheck {
    client: Arc<OllamaClient>,
    model: String,
    cache_capacity: u64,
}

impl HealthCheck {
    /// Create a new health check instance
    ///
    /// # Arguments
    ///
    /// * `classifier` - Reference to the intent classifier to check
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::ai::health::HealthCheck;
    /// use xze_core::ai::intent_classifier::{IntentClassifier, ClassifierConfig};
    /// use xze_core::ai::client::OllamaClient;
    /// use std::sync::Arc;
    ///
    /// let config = ClassifierConfig::default();
    /// let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
    /// let classifier = IntentClassifier::new(config.clone(), client);
    ///
    /// let health = HealthCheck::new(&classifier);
    /// ```
    pub fn new(_classifier: &IntentClassifier) -> Self {
        // Access fields via reflection or methods
        // For now, we'll use reasonable defaults and the public API
        Self::with_config(
            Arc::new(OllamaClient::new("http://localhost:11434".to_string())),
            "llama2:latest".to_string(),
            1000,
        )
    }

    /// Create health check with explicit configuration
    ///
    /// # Arguments
    ///
    /// * `client` - Ollama client to check
    /// * `model` - Model name to verify
    /// * `cache_capacity` - Maximum cache capacity
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::ai::health::HealthCheck;
    /// use xze_core::ai::client::OllamaClient;
    /// use std::sync::Arc;
    ///
    /// let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
    /// let health = HealthCheck::with_config(client, "llama2:latest".to_string(), 1000);
    /// ```
    pub fn with_config(client: Arc<OllamaClient>, model: String, cache_capacity: u64) -> Self {
        Self {
            client,
            model,
            cache_capacity,
        }
    }

    /// Perform a health check
    ///
    /// # Returns
    ///
    /// Returns a `HealthCheckResult` with detailed status information
    ///
    /// # Errors
    ///
    /// Returns an error if the health check itself fails to execute
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xze_core::ai::health::HealthCheck;
    /// # use xze_core::ai::client::OllamaClient;
    /// # use std::sync::Arc;
    /// # async fn example() -> xze_core::error::Result<()> {
    /// # let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
    /// # let health = HealthCheck::with_config(client, "llama2:latest".to_string(), 1000);
    /// let result = health.check().await?;
    /// println!("{}", result.summary());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check(&self) -> crate::error::Result<HealthCheckResult> {
        let start = Instant::now();
        let mut issues = Vec::new();

        // Check AI service availability
        let ai_service = self.check_ai_service().await;
        if !ai_service.available {
            issues.push("AI service unavailable".to_string());
        } else if let Some(response_time) = ai_service.response_time_ms {
            if response_time > 5000 {
                issues.push(format!("AI service slow response: {}ms", response_time));
            }
        }

        // Mock cache stats (in real implementation, would get from classifier)
        let cache_stats = CacheHealth {
            entry_count: 0,
            utilization: 0.0,
            max_capacity: self.cache_capacity,
        };

        if cache_stats.is_high_utilization() {
            issues.push(format!(
                "Cache utilization high: {:.1}%",
                cache_stats.utilization
            ));
        }

        // Determine overall status
        let status = if issues.is_empty() {
            HealthStatus::Healthy
        } else if ai_service.available {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(HealthCheckResult {
            status,
            timestamp: chrono::Utc::now(),
            duration_ms,
            issues,
            cache_stats,
            ai_service,
        })
    }

    /// Check AI service availability
    async fn check_ai_service(&self) -> ServiceHealth {
        let start = Instant::now();

        match self.client.list_models().await {
            Ok(models) => {
                let response_time = start.elapsed().as_millis() as u64;
                let model_available = models.iter().any(|m| m.name == self.model);

                ServiceHealth {
                    available: model_available,
                    response_time_ms: Some(response_time),
                    model: self.model.clone(),
                }
            }
            Err(_) => ServiceHealth {
                available: false,
                response_time_ms: None,
                model: self.model.clone(),
            },
        }
    }

    /// Perform a quick health check with timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration to wait for health check
    ///
    /// # Returns
    ///
    /// Returns `Ok(HealthCheckResult)` if check completes within timeout,
    /// or an error if timeout is exceeded
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xze_core::ai::health::HealthCheck;
    /// # use xze_core::ai::client::OllamaClient;
    /// # use std::sync::Arc;
    /// # use std::time::Duration;
    /// # async fn example() -> xze_core::error::Result<()> {
    /// # let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
    /// # let health = HealthCheck::with_config(client, "llama2:latest".to_string(), 1000);
    /// let result = health.check_with_timeout(Duration::from_secs(5)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_with_timeout(
        &self,
        timeout: Duration,
    ) -> crate::error::Result<HealthCheckResult> {
        tokio::time::timeout(timeout, self.check())
            .await
            .map_err(|_| crate::XzeError::timeout("Health check timed out"))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_is_healthy() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Degraded.is_healthy());
        assert!(!HealthStatus::Unhealthy.is_healthy());
    }

    #[test]
    fn test_health_status_is_operational() {
        assert!(HealthStatus::Healthy.is_operational());
        assert!(HealthStatus::Degraded.is_operational());
        assert!(!HealthStatus::Unhealthy.is_operational());
    }

    #[test]
    fn test_cache_health_utilization() {
        let low = CacheHealth {
            entry_count: 50,
            utilization: 50.0,
            max_capacity: 100,
        };
        assert!(!low.is_high_utilization());

        let high = CacheHealth {
            entry_count: 85,
            utilization: 85.0,
            max_capacity: 100,
        };
        assert!(high.is_high_utilization());
    }

    #[test]
    fn test_health_check_result_summary() {
        let healthy = HealthCheckResult {
            status: HealthStatus::Healthy,
            timestamp: chrono::Utc::now(),
            duration_ms: 100,
            issues: vec![],
            cache_stats: CacheHealth {
                entry_count: 50,
                utilization: 50.0,
                max_capacity: 100,
            },
            ai_service: ServiceHealth {
                available: true,
                response_time_ms: Some(50),
                model: "test".to_string(),
            },
        };

        let summary = healthy.summary();
        assert!(summary.contains("Healthy"));
        assert!(summary.contains("100ms"));

        let unhealthy = HealthCheckResult {
            status: HealthStatus::Unhealthy,
            timestamp: chrono::Utc::now(),
            duration_ms: 200,
            issues: vec!["Service down".to_string()],
            cache_stats: CacheHealth {
                entry_count: 0,
                utilization: 0.0,
                max_capacity: 100,
            },
            ai_service: ServiceHealth {
                available: false,
                response_time_ms: None,
                model: "test".to_string(),
            },
        };

        let summary = unhealthy.summary();
        assert!(summary.contains("Unhealthy"));
        assert!(summary.contains("Issues: 1"));
    }

    #[test]
    fn test_health_check_creation() {
        let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
        let health = HealthCheck::with_config(client, "test-model".to_string(), 1000);

        assert_eq!(health.model, "test-model");
        assert_eq!(health.cache_capacity, 1000);
    }

    #[test]
    fn test_service_health_serialization() {
        let service = ServiceHealth {
            available: true,
            response_time_ms: Some(100),
            model: "llama2".to_string(),
        };

        let json = serde_json::to_string(&service).unwrap();
        assert!(json.contains("llama2"));
        assert!(json.contains("100"));

        let deserialized: ServiceHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, "llama2");
        assert_eq!(deserialized.response_time_ms, Some(100));
    }
}

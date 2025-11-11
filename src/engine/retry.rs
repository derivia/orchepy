use crate::models::step::{BackoffStrategy, RetryConfig};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut attempt = 1;

        loop {
            debug!("Attempt {}/{}", attempt, self.config.max_attempts);

            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if attempt >= self.config.max_attempts {
                        warn!(
                            "All {} attempts failed. Last error: {}",
                            self.config.max_attempts, err
                        );
                        return Err(err);
                    }

                    let delay = self.calculate_delay(attempt);
                    warn!(
                        "Attempt {} failed: {}. Retrying in {:?}",
                        attempt, err, delay
                    );

                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay_ms;

        let delay_ms = match self.config.backoff {
            BackoffStrategy::Fixed => base_delay,
            BackoffStrategy::Exponential => {
                let multiplier = 2u64.pow(attempt - 1);
                base_delay.saturating_mul(multiplier)
            }
        };

        Duration::from_millis(delay_ms.min(60_000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let config = RetryConfig {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed,
            initial_delay_ms: 10,
        };

        let executor = RetryExecutor::new(config);
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

        let counter_clone = counter.clone();
        let result = executor
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if count < 2 {
                        Err("Temporary error")
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result, Ok(42));
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let config = RetryConfig {
            max_attempts: 2,
            backoff: BackoffStrategy::Fixed,
            initial_delay_ms: 10,
        };

        let executor = RetryExecutor::new(config);

        let result: Result<(), &str> = executor.execute(|| async { Err("Permanent error") }).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_exponential_backoff() {
        let config = RetryConfig {
            max_attempts: 5,
            backoff: BackoffStrategy::Exponential,
            initial_delay_ms: 1000,
        };

        let executor = RetryExecutor::new(config);

        assert_eq!(executor.calculate_delay(1), Duration::from_millis(1000));
        assert_eq!(executor.calculate_delay(2), Duration::from_millis(2000));
        assert_eq!(executor.calculate_delay(3), Duration::from_millis(4000));
        assert_eq!(executor.calculate_delay(4), Duration::from_millis(8000));
    }
}

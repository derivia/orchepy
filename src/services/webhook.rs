use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::Case;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseWebhookPayload {
    pub action: String,

    pub data: CaseWebhookData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseWebhookData {
    pub case_id: Uuid,

    pub workflow_id: Uuid,

    pub from_phase: Option<String>,

    pub to_phase: String,

    pub case_data: serde_json::Value,

    pub metadata: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct WebhookSender {
    client: Client,
}

impl WebhookSender {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub async fn send_case_moved(
        &self,
        webhook_url: &str,
        case: &Case,
        from_phase: Option<String>,
    ) -> Result<()> {
        let payload = CaseWebhookPayload {
            action: "case.moved".to_string(),
            data: CaseWebhookData {
                case_id: case.id,
                workflow_id: case.workflow_id,
                from_phase,
                to_phase: case.current_phase.clone(),
                case_data: case.data.clone(),
                metadata: case.metadata.clone(),
            },
        };

        info!(
            "Sending webhook to {}: case {} moved to phase '{}'",
            webhook_url, case.id, case.current_phase
        );

        match self.client.post(webhook_url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!(
                        "Webhook sent successfully: {} (status: {})",
                        webhook_url,
                        response.status()
                    );
                    Ok(())
                } else {
                    warn!(
                        "Webhook failed with status {}: {}",
                        response.status(),
                        webhook_url
                    );
                    Err(anyhow::anyhow!(
                        "Webhook returned status {}",
                        response.status()
                    ))
                }
            }
            Err(err) => {
                error!("Failed to send webhook to {}: {}", webhook_url, err);
                Err(anyhow::anyhow!("Webhook request failed: {}", err))
            }
        }
    }

    pub async fn send_case_moved_with_retry(
        &self,
        webhook_url: &str,
        case: &Case,
        from_phase: Option<String>,
        max_retries: u32,
    ) -> Result<()> {
        let mut attempts = 0;

        loop {
            attempts += 1;

            match self
                .send_case_moved(webhook_url, case, from_phase.clone())
                .await
            {
                Ok(_) => return Ok(()),
                Err(err) => {
                    if attempts >= max_retries {
                        error!("Webhook failed after {} attempts: {}", max_retries, err);
                        return Err(err);
                    }

                    warn!(
                        "Webhook attempt {}/{} failed, retrying in {}s: {}",
                        attempts, max_retries, attempts, err
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(2_u64.pow(attempts - 1)))
                        .await;
                }
            }
        }
    }
}

impl Default for WebhookSender {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = CaseWebhookPayload {
            action: "case.moved".to_string(),
            data: CaseWebhookData {
                case_id: Uuid::new_v4(),
                workflow_id: Uuid::new_v4(),
                from_phase: Some("OCR".to_string()),
                to_phase: "Validation".to_string(),
                case_data: serde_json::json!({"invoice_number": "123"}),
                metadata: None,
            },
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("case.moved"));
        assert!(json.contains("Validation"));
    }
}

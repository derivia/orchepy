use crate::engine::retry::RetryExecutor;
use crate::models::{
    execution::{Execution, ExecutionStatus, StepExecutionStatus, StepStatus},
    step::{FailureAction, Step, StepType},
    Event, Flow,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

pub struct Executor {
    http_client: Client,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            http_client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub async fn execute(&self, flow: &Flow, event: &Event) -> Result<Execution> {
        let mut execution = Execution::new(flow.id, event.id);
        execution.status = ExecutionStatus::Running;

        info!(
            "Starting execution {} for flow '{}' (event: {})",
            execution.id, flow.name, event.event_type
        );

        let mut steps_status: HashMap<String, StepStatus> = HashMap::new();
        let mut flow_failed = false;

        for step in &flow.steps {
            execution.current_step = Some(step.name.clone());

            info!("Executing step: {}", step.name);

            let step_result = self.execute_step(step, event, &steps_status).await;

            let status = match &step_result {
                Ok(response) => StepStatus {
                    status: StepExecutionStatus::Completed,
                    started_at: Utc::now(),
                    completed_at: Some(Utc::now()),
                    attempts: 1,
                    response: Some(response.clone()),
                    error: None,
                },
                Err(err) => {
                    let error_msg = err.to_string();
                    warn!("Step '{}' failed: {}", step.name, error_msg);

                    StepStatus {
                        status: StepExecutionStatus::Failed,
                        started_at: Utc::now(),
                        completed_at: Some(Utc::now()),
                        attempts: 1,
                        response: None,
                        error: Some(error_msg.clone()),
                    }
                }
            };

            steps_status.insert(step.name.clone(), status);

            if step_result.is_err() {
                match step.on_failure {
                    FailureAction::Stop => {
                        error!("Step '{}' failed. Stopping flow.", step.name);
                        flow_failed = true;
                        execution.error = step_result.err().map(|e| e.to_string());
                        break;
                    }
                    FailureAction::Continue => {
                        warn!("Step '{}' failed but continuing to next step.", step.name);
                    }
                }
            }
        }

        execution.steps_status = serde_json::to_value(&steps_status)?;
        execution.status = if flow_failed {
            ExecutionStatus::Failed
        } else {
            ExecutionStatus::Completed
        };
        execution.completed_at = Some(Utc::now());

        info!(
            "Execution {} finished with status: {:?}",
            execution.id, execution.status
        );

        Ok(execution)
    }

    fn execute_step<'a>(
        &'a self,
        step: &'a Step,
        event: &'a Event,
        previous_steps: &'a HashMap<String, StepStatus>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + 'a>> {
        Box::pin(async move { self.execute_step_inner(step, event, previous_steps).await })
    }

    async fn execute_step_inner(
        &self,
        step: &Step,
        event: &Event,
        previous_steps: &HashMap<String, StepStatus>,
    ) -> Result<Value> {
        match &step.step_type {
            StepType::Webhook {
                url,
                method,
                headers,
                body_template,
                timeout_ms,
                retry,
            } => {
                self.execute_webhook(
                    url,
                    method,
                    headers,
                    body_template,
                    event,
                    previous_steps,
                    *timeout_ms,
                    retry.as_ref(),
                )
                .await
            }

            StepType::Condition {
                condition,
                if_true,
                if_false,
            } => {
                let result = self.evaluate_condition(condition, event)?;
                let branch = if result { if_true } else { if_false };
                Box::pin(self.execute_step_inner(branch, event, previous_steps)).await
            }

            StepType::Delay { duration_ms } => {
                debug!("Delaying for {}ms", duration_ms);
                sleep(Duration::from_millis(*duration_ms)).await;
                Ok(json!({"delayed_ms": duration_ms}))
            }
        }
    }

    async fn execute_webhook(
        &self,
        url: &str,
        method: &str,
        headers: &HashMap<String, String>,
        body_template: &Value,
        event: &Event,
        previous_steps: &HashMap<String, StepStatus>,
        timeout_ms: Option<u64>,
        retry_config: Option<&crate::models::step::RetryConfig>,
    ) -> Result<Value> {
        let body = self.interpolate_template(body_template, event, previous_steps)?;

        let interpolated_url = self.interpolate_string(url, event, previous_steps)?;

        let operation = || async {
            let mut request = match method.to_uppercase().as_str() {
                "GET" => self.http_client.get(&interpolated_url),
                "POST" => self.http_client.post(&interpolated_url).json(&body),
                "PUT" => self.http_client.put(&interpolated_url).json(&body),
                "DELETE" => self.http_client.delete(&interpolated_url),
                "PATCH" => self.http_client.patch(&interpolated_url).json(&body),
                _ => return Err(anyhow!("Unsupported HTTP method: {}", method)),
            };

            for (key, value) in headers {
                let interpolated_value = self.interpolate_string(value, event, previous_steps)?;
                request = request.header(key, interpolated_value);
            }

            if let Some(timeout) = timeout_ms {
                request = request.timeout(Duration::from_millis(timeout));
            }

            let response = request.send().await.map_err(|e| anyhow!(e))?;

            let status = response.status();
            let body = response.text().await.map_err(|e| anyhow!(e))?;

            if !status.is_success() {
                return Err(anyhow!("HTTP {} - {}", status, body));
            }

            let result = serde_json::from_str::<Value>(&body).unwrap_or(json!({
                "status": status.as_u16(),
                "body": body
            }));

            Ok(result)
        };

        if let Some(retry) = retry_config {
            let executor = RetryExecutor::new(retry.clone());
            executor.execute(operation).await
        } else {
            operation().await
        }
    }

    fn evaluate_condition(&self, condition: &str, event: &Event) -> Result<bool> {
        if let Some((left, right)) = condition.split_once('>') {
            let left_val = self.extract_value(left.trim(), event)?;
            let right_val = self.extract_value(right.trim(), event)?;

            if let (Some(l), Some(r)) = (left_val.as_f64(), right_val.as_f64()) {
                return Ok(l > r);
            }
        }

        warn!("Could not evaluate condition: {}", condition);
        Ok(false)
    }

    fn extract_value(&self, expr: &str, event: &Event) -> Result<Value> {
        if let Some(path) = expr.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
            if let Some(field) = path.strip_prefix("event.data.") {
                return Ok(event.data.get(field).cloned().unwrap_or(Value::Null));
            }
        }

        if let Ok(num) = expr.parse::<f64>() {
            Ok(json!(num))
        } else {
            Ok(json!(expr))
        }
    }

    fn interpolate_template(
        &self,
        template: &Value,
        event: &Event,
        _previous_steps: &HashMap<String, StepStatus>,
    ) -> Result<Value> {
        match template {
            Value::String(s) => Ok(json!(self.interpolate_string(s, event, _previous_steps)?)),
            Value::Object(map) => {
                let mut result = serde_json::Map::new();
                for (key, value) in map {
                    result.insert(
                        key.clone(),
                        self.interpolate_template(value, event, _previous_steps)?,
                    );
                }
                Ok(Value::Object(result))
            }
            Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(self.interpolate_template(item, event, _previous_steps)?);
                }
                Ok(Value::Array(result))
            }
            other => Ok(other.clone()),
        }
    }

    fn interpolate_string(
        &self,
        template: &str,
        event: &Event,
        _previous_steps: &HashMap<String, StepStatus>,
    ) -> Result<String> {
        let mut result = template.to_string();

        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var = &result[start + 2..start + end];
                let value = if let Some(field) = var.strip_prefix("event.data.") {
                    event
                        .data
                        .get(field)
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            Value::Number(n) => Some(n.to_string()),
                            Value::Bool(b) => Some(b.to_string()),
                            _ => None,
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                result.replace_range(start..start + end + 1, &value);
            } else {
                break;
            }
        }

        Ok(result)
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

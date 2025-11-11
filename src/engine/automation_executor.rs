use crate::models::automation::{AutomationAction, AutomationResult, CaseModification, OnError, PhaseAutomation};
use crate::models::Case;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

pub struct AutomationExecutor {
    http_client: Client,
}

impl AutomationExecutor {
    pub fn new() -> Self {
        Self {
            http_client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub async fn execute_automations(
        &self,
        automations: &[&PhaseAutomation],
        case: &Case,
        from_phase: Option<&str>,
    ) -> Result<AutomationResult> {
        let mut result = AutomationResult::default();
        for automation in automations {
            info!(
                "Executing automation for phase '{}' (trigger: {:?})",
                automation.phase, automation.trigger
            );

            match self
                .execute_actions(&automation.actions, case, from_phase)
                .await
            {
                Ok(action_result) => {
                    result.modifications.extend(action_result.modifications);
                }
                Err(e) => {
                    error!(
                        "Failed to execute automation for phase '{}': {}",
                        automation.phase, e
                    );
                    return Err(e);
                }
            }
        }

        Ok(result)
    }

    fn execute_actions<'a>(
        &'a self,
        actions: &'a [AutomationAction],
        case: &'a Case,
        from_phase: Option<&'a str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AutomationResult>> + Send + 'a>> {
        Box::pin(async move {
        let mut action_responses: HashMap<String, Value> = HashMap::new();
        let mut result = AutomationResult::default();

        for (idx, action) in actions.iter().enumerate() {
            let action_name = action
                .name()
                .unwrap_or(&format!("action_{}", idx))
                .to_string();

            info!("Executing action: {}", action_name);

            let action_result = self
                .execute_action(action, case, from_phase, &action_responses)
                .await;

            match action_result {
                Ok((response, modifications)) => {
                    if let Some(id) = action.id() {
                        action_responses.insert(id.to_string(), response);
                    }
                    result.modifications.extend(modifications);
                }
                Err(e) => {
                    error!("Action '{}' failed: {}", action_name, e);

                    match action.on_error() {
                        OnError::Stop => {
                            return Err(anyhow!("Action '{}' failed: {}", action_name, e));
                        }
                        OnError::Continue => {
                            warn!("Action '{}' failed but continuing execution", action_name);
                        }
                    }
                }
            }
        }

        Ok(result)
        })
    }

    async fn execute_action(
        &self,
        action: &AutomationAction,
        case: &Case,
        from_phase: Option<&str>,
        previous_responses: &HashMap<String, Value>,
    ) -> Result<(Value, Vec<CaseModification>)> {
        match action {
            AutomationAction::Webhook {
                url,
                method,
                headers,
                fields,
                use_response_from,
                retry,
                ..
            } => {
                let body = if let Some(response_id) = use_response_from {
                    previous_responses
                        .get(response_id)
                        .cloned()
                        .ok_or_else(|| anyhow!("Response from '{}' not found", response_id))?
                } else {
                    self.build_webhook_body(case, from_phase, fields.as_ref())
                };

                let response = if retry.enabled {
                    self.execute_webhook_with_retry(
                        url,
                        method.as_deref().unwrap_or("POST"),
                        headers.as_ref(),
                        &body,
                        retry.max_attempts,
                        retry.delay_ms,
                    )
                    .await?
                } else {
                    self.execute_webhook(
                        url,
                        method.as_deref().unwrap_or("POST"),
                        headers.as_ref(),
                        &body,
                    )
                    .await?
                };
                Ok((response, vec![]))
            }

            AutomationAction::Delay { duration_ms, .. } => {
                debug!("Delaying for {}ms", duration_ms);
                sleep(Duration::from_millis(*duration_ms)).await;
                Ok((json!({"delayed_ms": duration_ms}), vec![]))
            }

            AutomationAction::Conditional {
                condition,
                then,
                r#else,
                ..
            } => {
                let condition_result = self.evaluate_condition(condition, case)?;

                let mut modifications = vec![];

                if condition_result {
                    debug!("Condition evaluated to true, executing then branch");
                    let result = self.execute_actions(then, case, from_phase).await?;
                    modifications.extend(result.modifications);
                } else if let Some(else_actions) = r#else {
                    debug!("Condition evaluated to false, executing else branch");
                    let result = self.execute_actions(else_actions, case, from_phase).await?;
                    modifications.extend(result.modifications);
                }

                Ok((json!({"condition_result": condition_result}), modifications))
            }

            AutomationAction::MoveToPhase { phase, .. } => {
                debug!("Queueing move to phase: {}", phase);
                Ok((
                    json!({"action": "move_to_phase", "phase": phase}),
                    vec![CaseModification::MoveToPhase { phase: phase.clone() }]
                ))
            }

            AutomationAction::SetField { field, value, .. } => {
                debug!("Queueing set field '{}' to {:?}", field, value);
                Ok((
                    json!({"action": "set_field", "field": field, "value": value}),
                    vec![CaseModification::SetField { field: field.clone(), value: value.clone() }]
                ))
            }
        }
    }

    fn evaluate_condition(&self, condition: &crate::models::automation::Condition, case: &Case) -> Result<bool> {
        use crate::models::automation::Condition;

        match condition {
            Condition::Simple { field, operator, value } => {
                self.evaluate_simple_condition(field, operator, value, case)
            }
            Condition::Complex { operator, conditions } => {
                use crate::models::automation::LogicalOperator;

                match operator {
                    LogicalOperator::And => {
                        for cond in conditions {
                            if !self.evaluate_simple_condition(&cond.field, &cond.operator, &cond.value, case)? {
                                return Ok(false);
                            }
                        }
                        Ok(true)
                    }
                    LogicalOperator::Or => {
                        for cond in conditions {
                            if self.evaluate_simple_condition(&cond.field, &cond.operator, &cond.value, case)? {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }
                }
            }
        }
    }

    fn evaluate_simple_condition(&self, field: &str, operator: &str, expected: &Value, case: &Case) -> Result<bool> {
        let actual_value = self.get_field_value(field, case)?;

        match operator {
            "==" | "=" => Ok(actual_value == *expected),
            "!=" => Ok(actual_value != *expected),
            ">" => {
                if let (Some(a), Some(b)) = (actual_value.as_f64(), expected.as_f64()) {
                    Ok(a > b)
                } else {
                    Err(anyhow!("Cannot compare non-numeric values with >"))
                }
            }
            "<" => {
                if let (Some(a), Some(b)) = (actual_value.as_f64(), expected.as_f64()) {
                    Ok(a < b)
                } else {
                    Err(anyhow!("Cannot compare non-numeric values with <"))
                }
            }
            ">=" => {
                if let (Some(a), Some(b)) = (actual_value.as_f64(), expected.as_f64()) {
                    Ok(a >= b)
                } else {
                    Err(anyhow!("Cannot compare non-numeric values with >="))
                }
            }
            "<=" => {
                if let (Some(a), Some(b)) = (actual_value.as_f64(), expected.as_f64()) {
                    Ok(a <= b)
                } else {
                    Err(anyhow!("Cannot compare non-numeric values with <="))
                }
            }
            "contains" => {
                if let Some(s) = actual_value.as_str() {
                    if let Some(substr) = expected.as_str() {
                        Ok(s.contains(substr))
                    } else {
                        Err(anyhow!("contains operator requires string expected value"))
                    }
                } else {
                    Err(anyhow!("contains operator requires string actual value"))
                }
            }
            _ => Err(anyhow!("Unsupported operator: {}", operator)),
        }
    }

    fn get_field_value(&self, field: &str, case: &Case) -> Result<Value> {
        let parts: Vec<&str> = field.split('.').collect();

        match parts.first() {
            Some(&"data") => {
                let mut current = &case.data;
                for part in &parts[1..] {
                    current = current.get(part).ok_or_else(|| anyhow!("Field '{}' not found", field))?;
                }
                Ok(current.clone())
            }
            Some(&"status") => Ok(json!(case.status)),
            Some(&"current_phase") => Ok(json!(case.current_phase)),
            Some(&"previous_phase") => Ok(json!(case.previous_phase)),
            _ => Err(anyhow!("Unsupported field path: {}", field)),
        }
    }

    fn build_webhook_body(
        &self,
        case: &Case,
        from_phase: Option<&str>,
        fields: Option<&Vec<String>>,
    ) -> Value {
        if let Some(field_list) = fields {
            let mut body = serde_json::Map::new();

            for field in field_list {
                match field.as_str() {
                    "case_id" | "id" => {
                        body.insert("case_id".to_string(), json!(case.id));
                    }
                    "workflow_id" => {
                        body.insert("workflow_id".to_string(), json!(case.workflow_id));
                    }
                    "current_phase" => {
                        body.insert("current_phase".to_string(), json!(case.current_phase));
                    }
                    "previous_phase" => {
                        body.insert("previous_phase".to_string(), json!(from_phase));
                    }
                    "data" => {
                        body.insert("data".to_string(), case.data.clone());
                    }
                    "metadata" => {
                        body.insert("metadata".to_string(), json!(case.metadata));
                    }
                    "status" => {
                        body.insert("status".to_string(), json!(case.status));
                    }
                    "created_at" => {
                        body.insert("created_at".to_string(), json!(case.created_at));
                    }
                    "updated_at" => {
                        body.insert("updated_at".to_string(), json!(case.updated_at));
                    }
                    _ => {
                        warn!("Unknown field '{}' requested in automation", field);
                    }
                }
            }

            Value::Object(body)
        } else {
            json!({
                "case_id": case.id,
                "workflow_id": case.workflow_id,
                "current_phase": case.current_phase,
                "previous_phase": from_phase,
                "data": case.data,
                "metadata": case.metadata,
                "status": case.status,
                "created_at": case.created_at,
                "updated_at": case.updated_at,
            })
        }
    }

    async fn execute_webhook(
        &self,
        url: &str,
        method: &str,
        headers: Option<&HashMap<String, String>>,
        body: &Value,
    ) -> Result<Value> {
        let mut request = match method.to_uppercase().as_str() {
            "GET" => self.http_client.get(url),
            "POST" => self.http_client.post(url).json(body),
            "PUT" => self.http_client.put(url).json(body),
            "DELETE" => self.http_client.delete(url),
            "PATCH" => self.http_client.patch(url).json(body),
            _ => return Err(anyhow!("Unsupported HTTP method: {}", method)),
        };

        if let Some(header_map) = headers {
            for (key, value) in header_map {
                request = request.header(key, value);
            }
        }

        let response = request.send().await.map_err(|e| anyhow!(e))?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| anyhow!(e))?;

        if !status.is_success() {
            return Err(anyhow!("HTTP {} - {}", status, body_text));
        }

        let result = serde_json::from_str::<Value>(&body_text).unwrap_or(json!({
            "status": status.as_u16(),
            "body": body_text
        }));

        Ok(result)
    }

    async fn execute_webhook_with_retry(
        &self,
        url: &str,
        method: &str,
        headers: Option<&HashMap<String, String>>,
        body: &Value,
        max_attempts: u32,
        delay_ms: u64,
    ) -> Result<Value> {
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            debug!("Webhook attempt {}/{} to {}", attempt, max_attempts, url);

            match self.execute_webhook(url, method, headers, body).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    warn!("Webhook attempt {} failed: {}", attempt, e);
                    last_error = Some(e);

                    if attempt < max_attempts {
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
    }
}

impl Default for AutomationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

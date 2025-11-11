use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepType {
    Webhook {
        url: String,
        method: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        body_template: serde_json::Value,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        retry: Option<RetryConfig>,
    },

    Condition {
        condition: String,

        if_true: Box<Step>,

        if_false: Box<Step>,
    },

    Delay {
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub name: String,

    #[serde(flatten)]
    pub step_type: StepType,

    #[serde(default)]
    pub on_failure: FailureAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FailureAction {
    #[default]
    Stop,

    Continue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,
}

fn default_initial_delay() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackoffStrategy {
    Fixed,

    Exponential,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential,
            initial_delay_ms: 1000,
        }
    }
}

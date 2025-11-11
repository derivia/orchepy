use crate::models::{Event, Flow};
use serde_json::Value;

pub struct Matcher;

impl Matcher {
    pub fn match_flows<'a>(event: &Event, flows: &'a [Flow]) -> Vec<&'a Flow> {
        flows
            .iter()
            .filter(|flow| Self::matches(event, flow))
            .collect()
    }

    fn matches(event: &Event, flow: &Flow) -> bool {
        if !flow.active {
            return false;
        }

        if event.event_type != flow.trigger.event_type {
            return false;
        }

        if !flow.trigger.filters.is_null() {
            return Self::check_filters(&event.data, &flow.trigger.filters);
        }

        true
    }

    fn check_filters(event_data: &Value, filters: &Value) -> bool {
        let Some(filter_obj) = filters.as_object() else {
            return true;
        };

        for (key, filter_value) in filter_obj {
            if let Some(field_name) = key.strip_suffix("_gt") {
                if !Self::check_gt(event_data, field_name, filter_value) {
                    return false;
                }
            } else if let Some(field_name) = key.strip_suffix("_lt") {
                if !Self::check_lt(event_data, field_name, filter_value) {
                    return false;
                }
            } else if let Some(field_name) = key.strip_suffix("_gte") {
                if !Self::check_gte(event_data, field_name, filter_value) {
                    return false;
                }
            } else if let Some(field_name) = key.strip_suffix("_lte") {
                if !Self::check_lte(event_data, field_name, filter_value) {
                    return false;
                }
            } else if let Some(field_name) = key.strip_suffix("_ne") {
                if !Self::check_ne(event_data, field_name, filter_value) {
                    return false;
                }
            } else {
                let event_value = event_data.get(key);
                if event_value != Some(filter_value) {
                    return false;
                }
            }
        }

        true
    }

    fn check_gt(data: &Value, field: &str, filter: &Value) -> bool {
        let Some(value) = data.get(field) else {
            return false;
        };
        compare_values(value, filter) == Some(std::cmp::Ordering::Greater)
    }

    fn check_lt(data: &Value, field: &str, filter: &Value) -> bool {
        let Some(value) = data.get(field) else {
            return false;
        };
        compare_values(value, filter) == Some(std::cmp::Ordering::Less)
    }

    fn check_gte(data: &Value, field: &str, filter: &Value) -> bool {
        let Some(value) = data.get(field) else {
            return false;
        };
        matches!(
            compare_values(value, filter),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        )
    }

    fn check_lte(data: &Value, field: &str, filter: &Value) -> bool {
        let Some(value) = data.get(field) else {
            return false;
        };
        matches!(
            compare_values(value, filter),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        )
    }

    fn check_ne(data: &Value, field: &str, filter: &Value) -> bool {
        let Some(value) = data.get(field) else {
            return false;
        };
        value != filter
    }
}

fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let a_f64 = a.as_f64()?;
            let b_f64 = b.as_f64()?;
            a_f64.partial_cmp(&b_f64)
        }
        (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

use chrono::Local;
use serde_json::{json, Value};

use super::types::{
    ClaudeDesktopCommonConfig, ClaudeDesktopInferenceModel, ClaudeDesktopProvider,
    ClaudeDesktopProviderInput,
};
use crate::coding::db_id::db_extract_id;

fn get_str(value: &Value, key: &str, default: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

fn get_opt_str(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn get_inference_models(value: &Value, key: &str) -> Vec<ClaudeDesktopInferenceModel> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("name")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(|name| ClaudeDesktopInferenceModel {
                            name: name.to_string(),
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn from_db_value_provider(value: Value) -> ClaudeDesktopProvider {
    ClaudeDesktopProvider {
        id: db_extract_id(&value),
        name: get_str(&value, "name", "Default"),
        inference_provider: get_str(&value, "inference_provider", "gateway"),
        inference_gateway_base_url: get_str(&value, "inference_gateway_base_url", ""),
        inference_gateway_api_key: get_str(&value, "inference_gateway_api_key", ""),
        inference_models: get_inference_models(&value, "inference_models"),
        notes: get_opt_str(&value, "notes"),
        sort_index: value
            .get("sort_index")
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        is_applied: value
            .get("is_applied")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        is_disabled: value
            .get("is_disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        created_at: get_str(&value, "created_at", ""),
        updated_at: get_str(&value, "updated_at", ""),
    }
}

pub fn provider_input_to_db_value(
    input: &ClaudeDesktopProviderInput,
    is_applied: bool,
    is_disabled: bool,
    created_at: String,
) -> Value {
    let now = Local::now().to_rfc3339();
    json!({
        "name": input.name,
        "inference_provider": input.inference_provider,
        "inference_gateway_base_url": input.inference_gateway_base_url,
        "inference_gateway_api_key": input.inference_gateway_api_key,
        "inference_models": input.inference_models,
        "notes": input.notes,
        "sort_index": input.sort_index,
        "is_applied": is_applied,
        "is_disabled": is_disabled,
        "created_at": created_at,
        "updated_at": now
    })
}

pub fn provider_to_db_value(provider: &ClaudeDesktopProvider) -> Value {
    json!({
        "name": provider.name,
        "inference_provider": provider.inference_provider,
        "inference_gateway_base_url": provider.inference_gateway_base_url,
        "inference_gateway_api_key": provider.inference_gateway_api_key,
        "inference_models": provider.inference_models,
        "notes": provider.notes,
        "sort_index": provider.sort_index,
        "is_applied": provider.is_applied,
        "is_disabled": provider.is_disabled,
        "created_at": provider.created_at,
        "updated_at": provider.updated_at
    })
}

pub fn from_db_value_common(value: Value) -> ClaudeDesktopCommonConfig {
    ClaudeDesktopCommonConfig {
        config: get_str(&value, "config", "{}"),
        root_dir: get_opt_str(&value, "root_dir"),
        updated_at: get_str(&value, "updated_at", ""),
    }
}

pub fn to_db_value_common(config: &str, root_dir: Option<&str>) -> Value {
    let mut value = json!({
        "config": config,
        "updated_at": Local::now().to_rfc3339()
    });

    if let Some(root_dir) = root_dir.filter(|value| !value.trim().is_empty()) {
        value["root_dir"] = json!(root_dir);
    }

    value
}

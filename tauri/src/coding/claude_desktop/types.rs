use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopInferenceModel {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopProvider {
    pub id: String,
    pub name: String,
    pub inference_provider: String,
    pub inference_gateway_base_url: String,
    pub inference_gateway_api_key: String,
    #[serde(default)]
    pub inference_models: Vec<ClaudeDesktopInferenceModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_index: Option<i32>,
    pub is_applied: bool,
    pub is_disabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopProviderInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub inference_provider: String,
    pub inference_gateway_base_url: String,
    pub inference_gateway_api_key: String,
    #[serde(default)]
    pub inference_models: Vec<ClaudeDesktopInferenceModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopCommonConfig {
    pub config: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopCommonConfigInput {
    pub config: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<String>,
    #[serde(default)]
    pub clear_root_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeDesktopPathInfo {
    pub path: String,
    pub source: String,
}

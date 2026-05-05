use chrono::Local;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::adapter;
use super::types::{
    ClaudeDesktopCommonConfig, ClaudeDesktopCommonConfigInput, ClaudeDesktopInferenceModel,
    ClaudeDesktopPathInfo, ClaudeDesktopProvider, ClaudeDesktopProviderInput,
};
use crate::coding::db_id::db_record_id;
use crate::db::DbState;
use tauri::Emitter;

const COMMON_CONFIG_ID: &str = "common";
const CLAUDE_DATA_DIR_NAME: &str = "Claude";
const CLAUDE_3P_DATA_DIR_NAME: &str = "Claude-3p";
const CONFIG_LIBRARY_DIR: &str = "configLibrary";
const META_FILE_NAME: &str = "_meta.json";
const DEFAULT_PROVIDER_NAME: &str = "Default";
const DEFAULT_INFERENCE_PROVIDER: &str = "gateway";
const DEVELOPER_SETTINGS_FILE_NAME: &str = "developer_settings.json";

fn new_profile_id() -> String {
    Uuid::new_v4().to_string()
}

fn is_valid_profile_id(id: &str) -> bool {
    id.len() == 36 && Uuid::parse_str(id).is_ok()
}

fn get_app_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir().ok_or_else(|| "Failed to resolve application data directory".to_string())
}

fn claude_desktop_candidates_from_data_dir(data_dir: &Path) -> Vec<PathBuf> {
    vec![
        claude_desktop_3p_root_dir_from_data_dir(data_dir),
        claude_desktop_app_root_dir_from_data_dir(data_dir),
    ]
}

fn claude_desktop_app_root_dir_from_data_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(CLAUDE_DATA_DIR_NAME)
}

fn claude_desktop_3p_root_dir_from_data_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(CLAUDE_3P_DATA_DIR_NAME)
}

fn get_claude_desktop_3p_root_dir() -> Result<PathBuf, String> {
    Ok(claude_desktop_3p_root_dir_from_data_dir(
        &get_app_data_dir()?
    ))
}

fn developer_settings_path_from_data_dir(data_dir: &Path) -> PathBuf {
    claude_desktop_app_root_dir_from_data_dir(data_dir).join(DEVELOPER_SETTINGS_FILE_NAME)
}

fn ensure_developer_mode_enabled(data_dir: &Path) -> Result<(), String> {
    let path = developer_settings_path_from_data_dir(data_dir);
    let mut settings = read_json_object(&path)?;
    settings.insert("allowDevTools".to_string(), Value::Bool(true));
    write_json_object(&path, &settings)
}

fn get_default_claude_desktop_root_dir() -> Result<PathBuf, String> {
    let data_dir = get_app_data_dir()?;
    for candidate in claude_desktop_candidates_from_data_dir(&data_dir) {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Ok(data_dir.join("Claude-3p"))
}

pub fn get_claude_desktop_mcp_config_path_sync() -> Result<PathBuf, String> {
    Ok(get_default_claude_desktop_root_dir()?.join("claude_desktop_config.json"))
}

async fn get_custom_root_dir_async(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
) -> Option<PathBuf> {
    let mut result = db
        .query("SELECT * OMIT id FROM claude_desktop_common_config:`common` LIMIT 1")
        .await
        .ok()?;
    let records: Vec<Value> = result.take(0).ok()?;
    let record = records.into_iter().next()?;
    let config = adapter::from_db_value_common(record);
    config
        .root_dir
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

pub async fn get_claude_desktop_root_dir_from_db_async(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
) -> Result<PathBuf, String> {
    if let Some(custom_root_dir) = get_custom_root_dir_async(db).await {
        return Ok(custom_root_dir);
    }
    get_default_claude_desktop_root_dir()
}

pub async fn get_claude_desktop_mcp_config_path_async(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
) -> Result<PathBuf, String> {
    Ok(get_claude_desktop_root_dir_from_db_async(db)
        .await?
        .join("claude_desktop_config.json"))
}

fn config_library_dir(root_dir: &Path) -> PathBuf {
    root_dir.join(CONFIG_LIBRARY_DIR)
}

fn meta_path(root_dir: &Path) -> PathBuf {
    config_library_dir(root_dir).join(META_FILE_NAME)
}

fn profile_path(root_dir: &Path, provider_id: &str) -> PathBuf {
    config_library_dir(root_dir).join(format!("{provider_id}.json"))
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>, String> {
    if !path.exists() {
        return Ok(Map::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read JSON file {}: {}", path.display(), error))?;
    if content.trim().is_empty() {
        return Ok(Map::new());
    }

    match serde_json::from_str::<Value>(&content)
        .map_err(|error| format!("Failed to parse JSON file {}: {}", path.display(), error))?
    {
        Value::Object(object) => Ok(object),
        _ => Err(format!("Expected JSON object in {}", path.display())),
    }
}

fn write_json_object(path: &Path, object: &Map<String, Value>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("Failed to create directory {}: {}", parent.display(), error)
        })?;
    }

    let serialized = serde_json::to_string_pretty(&Value::Object(object.clone()))
        .map_err(|error| format!("Failed to serialize JSON: {}", error))?;
    fs::write(path, format!("{serialized}\n"))
        .map_err(|error| format!("Failed to write JSON file {}: {}", path.display(), error))
}

fn inference_models_from_profile(profile: &Map<String, Value>) -> Vec<ClaudeDesktopInferenceModel> {
    profile
        .get("inferenceModels")
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

fn read_meta(root_dir: &Path) -> Result<Map<String, Value>, String> {
    read_json_object(&meta_path(root_dir))
}

fn read_applied_id(root_dir: &Path) -> Result<Option<String>, String> {
    Ok(read_meta(root_dir)?
        .get("appliedId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string))
}

fn upsert_meta_entry(
    root_dir: &Path,
    provider_id: &str,
    provider_name: &str,
    apply: bool,
) -> Result<(), String> {
    let mut meta = read_meta(root_dir)?;
    let entries_value = meta
        .remove("entries")
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let mut entries = entries_value.as_array().cloned().unwrap_or_default();
    let mut found = false;

    for entry in &mut entries {
        if entry.get("id").and_then(Value::as_str) == Some(provider_id) {
            if let Some(object) = entry.as_object_mut() {
                object.insert("name".to_string(), Value::String(provider_name.to_string()));
            }
            found = true;
            break;
        }
    }

    if !found {
        entries.push(json!({
            "id": provider_id,
            "name": provider_name
        }));
    }

    if apply {
        meta.insert(
            "appliedId".to_string(),
            Value::String(provider_id.to_string()),
        );
    }
    meta.insert("entries".to_string(), Value::Array(entries));
    write_json_object(&meta_path(root_dir), &meta)
}

fn remove_meta_entry(root_dir: &Path, provider_id: &str) -> Result<(), String> {
    let mut meta = read_meta(root_dir)?;
    let applied_id = meta.get("appliedId").and_then(Value::as_str);
    if applied_id == Some(provider_id) {
        meta.remove("appliedId");
    }

    let entries_value = meta
        .remove("entries")
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let entries = entries_value
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.get("id").and_then(Value::as_str) != Some(provider_id))
        .collect::<Vec<_>>();
    meta.insert("entries".to_string(), Value::Array(entries));
    write_json_object(&meta_path(root_dir), &meta)
}

fn replace_meta_entry_id(root_dir: &Path, old_id: &str, new_id: &str) -> Result<(), String> {
    let mut meta = read_meta(root_dir)?;
    if meta.get("appliedId").and_then(Value::as_str) == Some(old_id) {
        meta.insert("appliedId".to_string(), Value::String(new_id.to_string()));
    }

    let entries_value = meta
        .remove("entries")
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let mut entries = entries_value.as_array().cloned().unwrap_or_default();
    for entry in &mut entries {
        if entry.get("id").and_then(Value::as_str) == Some(old_id) {
            if let Some(object) = entry.as_object_mut() {
                object.insert("id".to_string(), Value::String(new_id.to_string()));
            }
        }
    }
    meta.insert("entries".to_string(), Value::Array(entries));
    write_json_object(&meta_path(root_dir), &meta)
}

#[cfg(test)]
fn write_provider_profile(root_dir: &Path, provider: &ClaudeDesktopProvider) -> Result<(), String> {
    let mut profile = read_json_object(&profile_path(root_dir, &provider.id))?;
    profile = merge_common_and_provider_profile(profile, None, provider)?;
    write_json_object(&profile_path(root_dir, &provider.id), &profile)
}

fn merge_common_and_provider_profile(
    mut profile: Map<String, Value>,
    common_config: Option<&str>,
    provider: &ClaudeDesktopProvider,
) -> Result<Map<String, Value>, String> {
    if let Some(common_config) = common_config {
        let common_value: Value = serde_json::from_str(common_config)
            .map_err(|error| format!("Failed to parse common config: {}", error))?;
        let Value::Object(common_object) = common_value else {
            return Err("Claude Desktop common config must be a JSON object".to_string());
        };
        for (key, value) in common_object {
            profile.insert(key, value);
        }
    }

    profile.insert(
        "inferenceProvider".to_string(),
        Value::String(provider.inference_provider.clone()),
    );
    profile.insert(
        "inferenceGatewayBaseUrl".to_string(),
        Value::String(provider.inference_gateway_base_url.clone()),
    );
    profile.insert(
        "inferenceGatewayApiKey".to_string(),
        Value::String(provider.inference_gateway_api_key.clone()),
    );
    profile.insert(
        "inferenceModels".to_string(),
        serde_json::to_value(&provider.inference_models)
            .map_err(|error| format!("Failed to serialize inference models: {}", error))?,
    );

    Ok(profile)
}

fn provider_from_profile(
    id: String,
    name: String,
    profile: Map<String, Value>,
    is_applied: bool,
    sort_index: i32,
) -> ClaudeDesktopProvider {
    let now = Local::now().to_rfc3339();
    ClaudeDesktopProvider {
        id,
        name,
        inference_provider: profile
            .get("inferenceProvider")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_INFERENCE_PROVIDER)
            .to_string(),
        inference_gateway_base_url: profile
            .get("inferenceGatewayBaseUrl")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        inference_gateway_api_key: profile
            .get("inferenceGatewayApiKey")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        inference_models: inference_models_from_profile(&profile),
        notes: None,
        sort_index: Some(sort_index),
        is_applied,
        is_disabled: false,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn hydrate_missing_profile_fields(
    root_dir: &Path,
    mut provider: ClaudeDesktopProvider,
) -> ClaudeDesktopProvider {
    if provider.inference_models.is_empty() {
        if let Ok(profile) = read_json_object(&profile_path(root_dir, &provider.id)) {
            provider.inference_models = inference_models_from_profile(&profile);
        }
    }

    provider
}

fn load_local_providers(root_dir: &Path) -> Result<Vec<ClaudeDesktopProvider>, String> {
    let meta = read_meta(root_dir)?;
    let applied_id = meta
        .get("appliedId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let entries = meta
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut providers = Vec::new();

    for (index, entry) in entries.into_iter().enumerate() {
        let Some(id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_PROVIDER_NAME)
            .to_string();
        let profile = read_json_object(&profile_path(root_dir, id))?;
        providers.push(provider_from_profile(
            id.to_string(),
            name,
            profile,
            applied_id.as_deref() == Some(id),
            index as i32,
        ));
    }

    if providers.is_empty() {
        let library_dir = config_library_dir(root_dir);
        if library_dir.exists() {
            for entry in fs::read_dir(&library_dir)
                .map_err(|error| format!("Failed to read configLibrary: {}", error))?
            {
                let entry =
                    entry.map_err(|error| format!("Failed to read config entry: {}", error))?;
                let path = entry.path();
                if path.file_name().and_then(|name| name.to_str()) == Some(META_FILE_NAME) {
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let Some(id) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                let profile = read_json_object(&path)?;
                providers.push(provider_from_profile(
                    id.to_string(),
                    DEFAULT_PROVIDER_NAME.to_string(),
                    profile,
                    applied_id.as_deref() == Some(id),
                    providers.len() as i32,
                ));
            }
        }
    }

    Ok(providers)
}

async fn seed_local_providers_if_needed(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    root_dir: &Path,
) -> Result<(), String> {
    let mut result = db
        .query("SELECT *, type::string(id) AS id FROM claude_desktop_provider LIMIT 1")
        .await
        .map_err(|error| format!("Failed to query Claude Desktop providers: {}", error))?;
    let existing: Vec<Value> = result
        .take(0)
        .map_err(|error| format!("Failed to parse provider records: {}", error))?;
    if !existing.is_empty() {
        return Ok(());
    }

    for provider in load_local_providers(root_dir)? {
        db.query(format!(
            "UPSERT {} CONTENT $data",
            db_record_id("claude_desktop_provider", &provider.id)
        ))
        .bind(("data", adapter::provider_to_db_value(&provider)))
        .await
        .map_err(|error| format!("Failed to seed local provider: {}", error))?;
    }

    Ok(())
}

async fn get_provider_by_id(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    provider_id: &str,
) -> Result<ClaudeDesktopProvider, String> {
    let record_id = db_record_id("claude_desktop_provider", provider_id);
    let provider: Option<Value> = db
        .query(format!(
            "SELECT *, type::string(id) AS id FROM {} LIMIT 1",
            record_id
        ))
        .await
        .map_err(|error| format!("Failed to query provider: {}", error))?
        .take(0)
        .map_err(|error| format!("Failed to parse provider: {}", error))?;
    provider
        .map(adapter::from_db_value_provider)
        .ok_or_else(|| "Provider not found".to_string())
}

async fn migrate_provider_id_if_needed(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    root_dir: &Path,
    provider: ClaudeDesktopProvider,
) -> Result<ClaudeDesktopProvider, String> {
    if is_valid_profile_id(&provider.id) {
        return Ok(provider);
    }

    let old_id = provider.id.clone();
    let mut new_id = new_profile_id();
    while profile_path(root_dir, &new_id).exists() {
        new_id = new_profile_id();
    }

    let old_path = profile_path(root_dir, &old_id);
    let new_path = profile_path(root_dir, &new_id);
    if old_path.exists() {
        fs::rename(&old_path, &new_path).map_err(|error| {
            format!(
                "Failed to rename Claude Desktop profile {} to {}: {}",
                old_path.display(),
                new_path.display(),
                error
            )
        })?;
    }

    replace_meta_entry_id(root_dir, &old_id, &new_id)?;

    let mut migrated = provider;
    migrated.id = new_id;
    migrated.updated_at = Local::now().to_rfc3339();

    db.query(format!(
        "UPSERT {} CONTENT $data",
        db_record_id("claude_desktop_provider", &migrated.id)
    ))
    .bind(("data", adapter::provider_to_db_value(&migrated)))
    .await
    .map_err(|error| format!("Failed to migrate provider ID: {}", error))?;
    db.query(format!(
        "DELETE {}",
        db_record_id("claude_desktop_provider", &old_id)
    ))
    .await
    .map_err(|error| format!("Failed to delete old provider ID: {}", error))?;

    Ok(migrated)
}

#[tauri::command]
pub async fn get_claude_desktop_config_path(
    state: tauri::State<'_, DbState>,
) -> Result<String, String> {
    let root_dir = get_claude_desktop_root_dir_from_db_async(&state.db()).await?;
    Ok(meta_path(&root_dir).to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_claude_desktop_root_path_info(
    state: tauri::State<'_, DbState>,
) -> Result<ClaudeDesktopPathInfo, String> {
    let db = state.db();
    if let Some(custom_root_dir) = get_custom_root_dir_async(&db).await {
        return Ok(ClaudeDesktopPathInfo {
            path: custom_root_dir.to_string_lossy().to_string(),
            source: "custom".to_string(),
        });
    }

    let root_dir = get_default_claude_desktop_root_dir()?;
    Ok(ClaudeDesktopPathInfo {
        path: root_dir.to_string_lossy().to_string(),
        source: "default".to_string(),
    })
}

#[tauri::command]
pub async fn get_claude_desktop_common_config(
    state: tauri::State<'_, DbState>,
) -> Result<Option<ClaudeDesktopCommonConfig>, String> {
    let mut result = state
        .db()
        .query("SELECT * OMIT id FROM claude_desktop_common_config:`common` LIMIT 1")
        .await
        .map_err(|error| format!("Failed to query common config: {}", error))?;
    let records: Vec<Value> = result
        .take(0)
        .map_err(|error| format!("Failed to parse common config: {}", error))?;
    Ok(records
        .into_iter()
        .next()
        .map(adapter::from_db_value_common))
}

#[tauri::command]
pub async fn save_claude_desktop_common_config(
    state: tauri::State<'_, DbState>,
    app: tauri::AppHandle,
    input: ClaudeDesktopCommonConfigInput,
) -> Result<(), String> {
    let _: Value =
        serde_json::from_str(&input.config).map_err(|error| format!("Invalid JSON: {}", error))?;
    let existing = get_claude_desktop_common_config(state.clone()).await?;
    let root_dir = if input.clear_root_dir {
        None
    } else {
        input
            .root_dir
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| existing.and_then(|config| config.root_dir))
    };

    state
        .db()
        .query(format!(
            "UPSERT {} CONTENT $data",
            db_record_id("claude_desktop_common_config", COMMON_CONFIG_ID)
        ))
        .bind((
            "data",
            adapter::to_db_value_common(&input.config, root_dir.as_deref()),
        ))
        .await
        .map_err(|error| format!("Failed to save common config: {}", error))?;
    let _ = app.emit("config-changed", "window");
    Ok(())
}

#[tauri::command]
pub async fn list_claude_desktop_providers(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ClaudeDesktopProvider>, String> {
    let db = state.db();
    let root_dir = get_claude_desktop_root_dir_from_db_async(&db).await?;
    seed_local_providers_if_needed(&db, &root_dir).await?;

    let mut result = db
        .query("SELECT *, type::string(id) AS id FROM claude_desktop_provider ORDER BY sort_index ASC, created_at ASC")
        .await
        .map_err(|error| format!("Failed to list providers: {}", error))?;
    let records: Vec<Value> = result
        .take(0)
        .map_err(|error| format!("Failed to parse providers: {}", error))?;
    let applied_id = read_applied_id(&root_dir)?;
    let mut providers = Vec::new();
    for record in records {
        let provider = adapter::from_db_value_provider(record);
        let original_id = provider.id.clone();
        let mut provider = migrate_provider_id_if_needed(&db, &root_dir, provider).await?;
        provider = hydrate_missing_profile_fields(&root_dir, provider);
        provider.is_applied = applied_id.as_deref() == Some(provider.id.as_str())
            || applied_id.as_deref() == Some(original_id.as_str());
        providers.push(provider);
    }

    Ok(providers)
}

#[tauri::command]
pub async fn create_claude_desktop_provider(
    state: tauri::State<'_, DbState>,
    provider: ClaudeDesktopProviderInput,
) -> Result<ClaudeDesktopProvider, String> {
    let db = state.db();
    let id = provider
        .id
        .as_deref()
        .filter(|id| is_valid_profile_id(id))
        .map(str::to_string)
        .unwrap_or_else(new_profile_id);
    let now = Local::now().to_rfc3339();
    let data = adapter::provider_input_to_db_value(&provider, false, false, now);

    db.query(format!(
        "UPSERT {} CONTENT $data",
        db_record_id("claude_desktop_provider", &id)
    ))
    .bind(("data", data))
    .await
    .map_err(|error| format!("Failed to create provider: {}", error))?;

    get_provider_by_id(&db, &id).await
}

#[tauri::command]
pub async fn update_claude_desktop_provider(
    state: tauri::State<'_, DbState>,
    app: tauri::AppHandle,
    provider: ClaudeDesktopProvider,
) -> Result<ClaudeDesktopProvider, String> {
    let db = state.db();
    let input = ClaudeDesktopProviderInput {
        id: Some(provider.id.clone()),
        name: provider.name.clone(),
        inference_provider: provider.inference_provider.clone(),
        inference_gateway_base_url: provider.inference_gateway_base_url.clone(),
        inference_gateway_api_key: provider.inference_gateway_api_key.clone(),
        inference_models: provider.inference_models.clone(),
        notes: provider.notes.clone(),
        sort_index: provider.sort_index,
    };
    let data = adapter::provider_input_to_db_value(
        &input,
        provider.is_applied,
        provider.is_disabled,
        provider.created_at.clone(),
    );

    db.query(format!(
        "UPSERT {} CONTENT $data",
        db_record_id("claude_desktop_provider", &provider.id)
    ))
    .bind(("data", data))
    .await
    .map_err(|error| format!("Failed to update provider: {}", error))?;

    let mut updated = get_provider_by_id(&db, &provider.id).await?;
    if updated.is_applied && !updated.is_disabled {
        let root_dir = get_claude_desktop_root_dir_from_db_async(&db).await?;
        updated = hydrate_missing_profile_fields(&root_dir, updated);
        let common_config = get_claude_desktop_common_config(state.clone())
            .await?
            .map(|value| value.config);
        let profile = read_json_object(&profile_path(&root_dir, &updated.id))?;
        let merged_profile =
            merge_common_and_provider_profile(profile, common_config.as_deref(), &updated)?;
        write_json_object(&profile_path(&root_dir, &updated.id), &merged_profile)?;
        upsert_meta_entry(&root_dir, &updated.id, &updated.name, true)?;
        let _ = app.emit("config-changed", "window");
    }
    Ok(updated)
}

#[tauri::command]
pub async fn delete_claude_desktop_provider(
    state: tauri::State<'_, DbState>,
    app: tauri::AppHandle,
    id: String,
) -> Result<(), String> {
    let db = state.db();
    let root_dir = get_claude_desktop_root_dir_from_db_async(&db).await?;
    db.query(format!(
        "DELETE {}",
        db_record_id("claude_desktop_provider", &id)
    ))
    .await
    .map_err(|error| format!("Failed to delete provider: {}", error))?;

    remove_meta_entry(&root_dir, &id)?;
    let path = profile_path(&root_dir, &id);
    if path.exists() {
        fs::remove_file(&path).map_err(|error| {
            format!(
                "Failed to remove profile file {}: {}",
                path.display(),
                error
            )
        })?;
    }
    let _ = app.emit("config-changed", "window");
    Ok(())
}

#[tauri::command]
pub async fn apply_claude_desktop_config(
    state: tauri::State<'_, DbState>,
    app: tauri::AppHandle,
    provider_id: String,
) -> Result<(), String> {
    let db = state.db();
    let provider = get_provider_by_id(&db, &provider_id).await?;
    if provider.is_disabled {
        return Err("Provider is disabled and cannot be applied".to_string());
    }

    let data_dir = get_app_data_dir()?;
    ensure_developer_mode_enabled(&data_dir)?;

    let root_dir = get_claude_desktop_3p_root_dir()?;
    let provider = migrate_provider_id_if_needed(&db, &root_dir, provider).await?;
    let provider = hydrate_missing_profile_fields(&root_dir, provider);
    let common_config = get_claude_desktop_common_config(state.clone())
        .await?
        .map(|value| value.config);
    let profile = read_json_object(&profile_path(&root_dir, &provider.id))?;
    let merged_profile =
        merge_common_and_provider_profile(profile, common_config.as_deref(), &provider)?;
    write_json_object(&profile_path(&root_dir, &provider.id), &merged_profile)?;
    upsert_meta_entry(&root_dir, &provider.id, &provider.name, true)?;
    let now = Local::now().to_rfc3339();

    db.query("UPDATE claude_desktop_provider SET is_applied = false, updated_at = $now WHERE is_applied = true")
        .bind(("now", now.clone()))
        .await
        .map_err(|error| format!("Failed to reset applied providers: {}", error))?;
    db.query(format!(
        "UPDATE {} SET is_applied = true, updated_at = $now",
        db_record_id("claude_desktop_provider", &provider.id)
    ))
    .bind(("now", now))
    .await
    .map_err(|error| format!("Failed to mark provider applied: {}", error))?;

    let _ = app.emit("config-changed", "window");
    Ok(())
}

#[tauri::command]
pub async fn toggle_claude_desktop_provider_disabled(
    state: tauri::State<'_, DbState>,
    provider_id: String,
    is_disabled: bool,
) -> Result<(), String> {
    let db = state.db();
    let provider = get_provider_by_id(&db, &provider_id).await?;
    if provider.is_applied && is_disabled {
        return Err("Applied provider cannot be disabled".to_string());
    }

    db.query(format!(
        "UPDATE {} SET is_disabled = $is_disabled, updated_at = $now",
        db_record_id("claude_desktop_provider", &provider_id)
    ))
    .bind(("is_disabled", is_disabled))
    .bind(("now", Local::now().to_rfc3339()))
    .await
    .map_err(|error| format!("Failed to toggle provider: {}", error))?;
    Ok(())
}

#[tauri::command]
pub async fn reorder_claude_desktop_providers(
    state: tauri::State<'_, DbState>,
    ids: Vec<String>,
) -> Result<(), String> {
    let db = state.db();
    for (index, id) in ids.iter().enumerate() {
        db.query(format!(
            "UPDATE {} SET sort_index = $sort_index, updated_at = $now",
            db_record_id("claude_desktop_provider", id)
        ))
        .bind(("sort_index", index as i32))
        .bind(("now", Local::now().to_rfc3339()))
        .await
        .map_err(|error| format!("Failed to reorder providers: {}", error))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_candidates_do_not_embed_user_absolute_path() {
        let data_dir = PathBuf::from("/tmp/Application Support");
        let candidates = claude_desktop_candidates_from_data_dir(&data_dir);

        assert_eq!(candidates[0], data_dir.join("Claude-3p"));
        assert_eq!(candidates[1], data_dir.join("Claude"));
    }

    #[test]
    fn developer_mode_file_is_created_under_claude_dir() {
        let dir = tempdir().expect("tempdir");
        let data_dir = dir.path();

        ensure_developer_mode_enabled(data_dir).expect("enable developer mode");
        let settings_path = developer_settings_path_from_data_dir(data_dir);
        let settings = read_json_object(&settings_path).expect("read developer settings");

        assert_eq!(
            settings_path,
            data_dir.join("Claude").join("developer_settings.json")
        );
        assert_eq!(settings.get("allowDevTools"), Some(&json!(true)));
    }

    #[test]
    fn developer_mode_write_preserves_unknown_fields() {
        let dir = tempdir().expect("tempdir");
        let data_dir = dir.path();
        let settings_path = developer_settings_path_from_data_dir(data_dir);
        fs::create_dir_all(settings_path.parent().unwrap()).expect("create Claude dir");
        fs::write(
            &settings_path,
            "{\n  \"allowDevTools\": false,\n  \"useMacMenuBarHelper\": true\n}\n",
        )
        .expect("seed developer settings");

        ensure_developer_mode_enabled(data_dir).expect("enable developer mode");
        let settings = read_json_object(&settings_path).expect("read developer settings");

        assert_eq!(settings.get("allowDevTools"), Some(&json!(true)));
        assert_eq!(settings.get("useMacMenuBarHelper"), Some(&json!(true)));
    }

    #[test]
    fn threep_config_library_path_uses_claude_3p_dir() {
        let data_dir = PathBuf::from("/tmp/Application Support");
        let root = claude_desktop_3p_root_dir_from_data_dir(&data_dir);

        assert_eq!(
            config_library_dir(&root),
            data_dir.join("Claude-3p").join("configLibrary")
        );
    }

    #[test]
    fn generated_profile_ids_are_claude_desktop_compatible() {
        let id = new_profile_id();

        assert!(is_valid_profile_id(&id));
        assert_eq!(id.len(), 36);
        assert!(id.contains('-'));
        assert!(!is_valid_profile_id("1cd847748f9947fa8c93dd0e0c7f0ee2"));
    }

    #[test]
    fn provider_profile_write_preserves_unknown_fields() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let provider = ClaudeDesktopProvider {
            id: "profile-a".to_string(),
            name: "Default".to_string(),
            inference_provider: "gateway".to_string(),
            inference_gateway_base_url: "https://example.com".to_string(),
            inference_gateway_api_key: "secret".to_string(),
            inference_models: vec![
                ClaudeDesktopInferenceModel {
                    name: "MiniMax-M2.7".to_string(),
                },
                ClaudeDesktopInferenceModel {
                    name: "kimi-k2.6".to_string(),
                },
            ],
            notes: None,
            sort_index: Some(0),
            is_applied: true,
            is_disabled: false,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        let path = profile_path(root, &provider.id);
        fs::create_dir_all(path.parent().unwrap()).expect("create configLibrary");
        fs::write(&path, "{\n  \"custom\": true\n}\n").expect("seed profile");

        write_provider_profile(root, &provider).expect("write profile");
        let profile = read_json_object(&path).expect("read profile");

        assert_eq!(profile.get("custom"), Some(&json!(true)));
        assert_eq!(
            profile.get("inferenceGatewayBaseUrl"),
            Some(&json!("https://example.com"))
        );
        assert_eq!(
            profile.get("inferenceGatewayApiKey"),
            Some(&json!("secret"))
        );
        assert_eq!(
            profile.get("inferenceModels"),
            Some(&json!([
                { "name": "MiniMax-M2.7" },
                { "name": "kimi-k2.6" }
            ]))
        );
    }

    #[test]
    fn common_config_merge_preserves_provider_specific_fields() {
        let provider = ClaudeDesktopProvider {
            id: "profile-a".to_string(),
            name: "Default".to_string(),
            inference_provider: "gateway".to_string(),
            inference_gateway_base_url: "https://provider.example.com".to_string(),
            inference_gateway_api_key: "provider-secret".to_string(),
            inference_models: vec![ClaudeDesktopInferenceModel {
                name: "provider-model".to_string(),
            }],
            notes: None,
            sort_index: Some(0),
            is_applied: true,
            is_disabled: false,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        let mut profile = Map::new();
        profile.insert("existing".to_string(), json!(true));

        let merged = merge_common_and_provider_profile(
            profile,
            Some(r#"{"shared":1,"inferenceGatewayBaseUrl":"https://common.example.com","inferenceModels":[{"name":"common-model"}]}"#),
            &provider,
        )
        .expect("merge profile");

        assert_eq!(merged.get("existing"), Some(&json!(true)));
        assert_eq!(merged.get("shared"), Some(&json!(1)));
        assert_eq!(
            merged.get("inferenceGatewayBaseUrl"),
            Some(&json!("https://provider.example.com"))
        );
        assert_eq!(
            merged.get("inferenceModels"),
            Some(&json!([{ "name": "provider-model" }]))
        );
    }

    #[test]
    fn meta_write_preserves_unknown_fields_and_sets_applied_id() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let path = meta_path(root);
        fs::create_dir_all(path.parent().unwrap()).expect("create configLibrary");
        fs::write(&path, "{\n  \"custom\": 1,\n  \"entries\": []\n}\n").expect("seed meta");

        upsert_meta_entry(root, "abc", "Default", true).expect("write meta");
        let meta = read_json_object(&path).expect("read meta");

        assert_eq!(meta.get("custom"), Some(&json!(1)));
        assert_eq!(meta.get("appliedId"), Some(&json!("abc")));
        assert_eq!(
            meta.get("entries")
                .and_then(Value::as_array)
                .and_then(|entries| entries.first())
                .and_then(|entry| entry.get("name")),
            Some(&json!("Default"))
        );
    }

    #[test]
    fn meta_id_replace_updates_entries_and_applied_id() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let path = meta_path(root);
        fs::create_dir_all(path.parent().unwrap()).expect("create configLibrary");
        fs::write(
            &path,
            "{\n  \"appliedId\": \"oldid\",\n  \"entries\": [{\"id\":\"oldid\",\"name\":\"Default\"}]\n}\n",
        )
        .expect("seed meta");

        replace_meta_entry_id(root, "oldid", "00000000-0000-4000-8000-000000000001")
            .expect("replace meta id");
        let meta = read_json_object(&path).expect("read meta");

        assert_eq!(
            meta.get("appliedId"),
            Some(&json!("00000000-0000-4000-8000-000000000001"))
        );
        assert_eq!(
            meta.get("entries")
                .and_then(Value::as_array)
                .and_then(|entries| entries.first())
                .and_then(|entry| entry.get("id")),
            Some(&json!("00000000-0000-4000-8000-000000000001"))
        );
    }
}

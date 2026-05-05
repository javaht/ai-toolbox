use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::utils::{
    extract_prompt_title_text, extract_text, join_safe_relative, parse_timestamp_to_ms,
    read_head_tail_lines, sanitize_path_segment, strip_path_prefix, text_contains_query,
    truncate_summary,
};
use super::{SessionMessage, SessionMeta};

const PROVIDER_ID: &str = "claude";

pub fn scan_sessions(root: &Path) -> Vec<SessionMeta> {
    let mut session_files = Vec::new();
    collect_session_files(root, &mut session_files);
    session_files
        .into_iter()
        .filter_map(|path| parse_session_file(&path))
        .collect()
}

pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let audit_path = audit_path_for_session_file(path);
    let file = File::open(&audit_path).map_err(|error| {
        format!(
            "Failed to open Claude Desktop audit file {}: {error}",
            audit_path.display()
        )
    })?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(value) => value,
            Err(_) => continue,
        };
        let value: Value = match serde_json::from_str(&line) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        let Some(role) = role_for_audit_entry(&value) else {
            continue;
        };
        let content = audit_entry_content(&value);
        if content.trim().is_empty() {
            continue;
        }
        let ts = value
            .get("timestamp")
            .or_else(|| value.get("_audit_timestamp"))
            .and_then(parse_timestamp_to_ms);
        messages.push(SessionMessage { role, content, ts });
    }

    Ok(messages)
}

pub fn scan_messages_for_query(path: &Path, query_lower: &str) -> Result<bool, String> {
    for message in load_messages(path)? {
        if text_contains_query(&message.content, query_lower) {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn delete_session(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_file(path).map_err(|error| {
            format!(
                "Failed to delete Claude Desktop session file {}: {error}",
                path.display()
            )
        })?;
    }

    let detail_dir = detail_dir_for_session_file(path);
    if detail_dir.exists() {
        std::fs::remove_dir_all(&detail_dir).map_err(|error| {
            format!(
                "Failed to delete Claude Desktop session directory {}: {error}",
                detail_dir.display()
            )
        })?;
    }

    Ok(())
}

pub fn export_native_snapshot(root: &Path, session_path: &Path) -> Result<Value, String> {
    let session_file_content = std::fs::read_to_string(session_path).map_err(|error| {
        format!(
            "Failed to read Claude Desktop session file {}: {error}",
            session_path.display()
        )
    })?;
    let relative_session_path = strip_path_prefix(root, session_path).ok_or_else(|| {
        format!(
            "Session path {} is outside Claude Desktop sessions root {}",
            session_path.display(),
            root.display()
        )
    })?;
    let audit_path = audit_path_for_session_file(session_path);
    let relative_audit_path = strip_path_prefix(root, &audit_path);
    let audit_file_content = if audit_path.exists() {
        Some(std::fs::read_to_string(&audit_path).map_err(|error| {
            format!(
                "Failed to read Claude Desktop audit file {}: {error}",
                audit_path.display()
            )
        })?)
    } else {
        None
    };

    Ok(serde_json::json!({
        "relativeSessionPath": relative_session_path,
        "sessionFileContent": session_file_content,
        "relativeAuditPath": relative_audit_path,
        "auditFileContent": audit_file_content
    }))
}

pub fn import_native_snapshot(
    root: &Path,
    session_id: &str,
    snapshot: &Value,
) -> Result<PathBuf, String> {
    let relative_session_path = snapshot
        .get("relativeSessionPath")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "imported/{}.json",
                sanitize_path_segment(session_id, "session")
            )
        });
    let session_file_content = snapshot
        .get("sessionFileContent")
        .and_then(Value::as_str)
        .ok_or_else(|| "Claude Desktop snapshot missing sessionFileContent".to_string())?;

    let target_path = join_safe_relative(root, &relative_session_path)?;
    if target_path.exists() {
        return Err(format!(
            "Claude Desktop session file already exists: {}",
            target_path.display()
        ));
    }
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create Claude Desktop session directory {}: {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&target_path, session_file_content).map_err(|error| {
        format!(
            "Failed to write Claude Desktop session file {}: {error}",
            target_path.display()
        )
    })?;

    if let Some(audit_file_content) = snapshot.get("auditFileContent").and_then(Value::as_str) {
        let audit_path = snapshot
            .get("relativeAuditPath")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(|relative| join_safe_relative(root, relative))
            .transpose()?
            .unwrap_or_else(|| audit_path_for_session_file(&target_path));
        if let Some(parent) = audit_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Failed to create Claude Desktop audit directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        std::fs::write(&audit_path, audit_file_content).map_err(|error| {
            format!(
                "Failed to write Claude Desktop audit file {}: {error}",
                audit_path.display()
            )
        })?;
    }

    Ok(target_path)
}

fn collect_session_files(root: &Path, files: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_session_files(&path, files);
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name.starts_with("local_") && file_name.ends_with(".json") {
            files.push(path);
        }
    }
}

fn parse_session_file(path: &Path) -> Option<SessionMeta> {
    let data = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&data).ok()?;
    let session_id = value
        .get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
        })?;
    let title = value
        .get("initialMessage")
        .and_then(Value::as_str)
        .and_then(|text| extract_prompt_title_text(text, 80))
        .or_else(|| {
            value
                .get("processName")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
        });
    let summary = read_audit_summary(path)
        .or_else(|| {
            value
                .get("initialMessage")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .map(|text| truncate_summary(&text, 160));
    let project_dir = value
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    let created_at = value
        .get("createdAt")
        .and_then(Value::as_i64)
        .or_else(|| value.get("created_at").and_then(parse_timestamp_to_ms));
    let last_active_at = value
        .get("lastActivityAt")
        .and_then(Value::as_i64)
        .or_else(|| value.get("updatedAt").and_then(parse_timestamp_to_ms))
        .or(created_at);

    Some(SessionMeta {
        provider_id: PROVIDER_ID.to_string(),
        session_id: session_id.clone(),
        title,
        summary,
        project_dir,
        created_at,
        last_active_at,
        source_path: path.to_string_lossy().to_string(),
        resume_command: None,
    })
}

fn read_audit_summary(path: &Path) -> Option<String> {
    let audit_path = audit_path_for_session_file(path);
    let (_head, tail) = read_head_tail_lines(&audit_path, 0, 40).ok()?;
    for line in tail.iter().rev() {
        let value: Value = serde_json::from_str(line).ok()?;
        if matches!(
            value.get("type").and_then(Value::as_str),
            Some("system") | Some("result")
        ) {
            continue;
        }
        let content = audit_entry_content(&value);
        if !content.trim().is_empty() {
            return Some(content);
        }
    }
    None
}

fn role_for_audit_entry(value: &Value) -> Option<String> {
    match value.get("type").and_then(Value::as_str) {
        Some("user") => Some("user".to_string()),
        Some("assistant") => Some("assistant".to_string()),
        Some("system") => None,
        Some("result") => None,
        Some(other) => Some(other.to_string()),
        None => value
            .get("message")
            .and_then(|message| message.get("role"))
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

fn audit_entry_content(value: &Value) -> String {
    value
        .get("message")
        .and_then(|message| message.get("content").map(extract_text))
        .or_else(|| {
            value
                .get("result")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default()
}

fn detail_dir_for_session_file(path: &Path) -> PathBuf {
    path.with_extension("")
}

fn audit_path_for_session_file(path: &Path) -> PathBuf {
    detail_dir_for_session_file(path).join("audit.jsonl")
}

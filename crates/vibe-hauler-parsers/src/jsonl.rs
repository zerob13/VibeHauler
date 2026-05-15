use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use vibe_hauler_core::{AgentSession, AppId, RiskLevel, SessionSource, TokenUsage, now_rfc3339};
use vibe_hauler_redaction::{RedactionPolicy, redact_text};

use crate::{ParserInput, ParserSupport, SessionParser};

#[derive(Clone, Copy, Debug, Default)]
pub struct JsonlSessionParser;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JsonlParseStats {
    pub known_events: u32,
    pub unknown_events: u32,
    pub malformed_lines: u32,
}

#[derive(Debug, Deserialize)]
struct JsonlEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    timestamp: Option<String>,
    cwd: Option<std::path::PathBuf>,
    content: Option<Value>,
    summary: Option<String>,
    usage: Option<UsageEvent>,
    payload: Option<Value>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct UsageEvent {
    input: Option<u64>,
    output: Option<u64>,
    total: Option<u64>,
}

impl SessionParser for JsonlSessionParser {
    fn name(&self) -> &'static str {
        "jsonl-session"
    }

    fn supports(&self, input: &ParserInput) -> ParserSupport {
        match input.path.extension().and_then(|ext| ext.to_str()) {
            Some("jsonl") => ParserSupport::Supported,
            Some("json") => ParserSupport::Maybe,
            _ => ParserSupport::Unsupported,
        }
    }

    fn parse(&self, input: ParserInput) -> anyhow::Result<Vec<AgentSession>> {
        let (session, _stats) = parse_jsonl_session(&input, &RedactionPolicy::default())?;
        Ok(vec![session])
    }
}

pub fn parse_jsonl_session(
    input: &ParserInput,
    redaction_policy: &RedactionPolicy,
) -> anyhow::Result<(AgentSession, JsonlParseStats)> {
    let file = File::open(&input.path)
        .with_context(|| format!("failed to open JSONL {}", input.path.display()))?;
    let metadata = file.metadata()?;
    let mut stats = JsonlParseStats::default();
    let mut started_at: Option<String> = None;
    let mut updated_at: Option<String> = None;
    let mut cwd = None;
    let mut title = None;
    let mut preview_parts = Vec::new();
    let mut token_usage = TokenUsage {
        input: None,
        output: None,
        total: None,
    };

    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<JsonlEvent>(&line) else {
            stats.malformed_lines += 1;
            continue;
        };
        let event_type = event.event_type.as_deref().unwrap_or("unknown");
        if matches!(event_type, "user" | "assistant" | "summary" | "system") {
            stats.known_events += 1;
        } else {
            stats.unknown_events += 1;
        }

        if let Some(timestamp) = event.timestamp.as_deref() {
            update_bounds(timestamp, &mut started_at, &mut updated_at);
        }
        if cwd.is_none() {
            cwd = event.cwd;
        }
        if let Some(usage) = event.usage {
            merge_usage(&mut token_usage, usage);
        }
        if let Some(summary) = event.summary {
            title.get_or_insert_with(|| first_words(&summary, 8));
            preview_parts.push(summary);
        }
        if let Some(content) = content_to_string(event.content.as_ref().or(event.payload.as_ref()))
        {
            if title.is_none() && event_type == "user" {
                title = Some(first_words(&content, 8));
            }
            if matches!(event_type, "user" | "assistant" | "summary") {
                preview_parts.push(content);
            }
        }
    }

    let raw_preview = preview_parts.join(" ");
    let preview = (!raw_preview.is_empty())
        .then(|| redact_text(&raw_preview, redaction_policy).value)
        .or_else(|| Some("No preview available".to_owned()));
    let preview_hash = preview.as_deref().map(hash_text);
    let app = input
        .app_hint
        .as_deref()
        .map_or(AppId::Codex, AppId::from_key);
    let updated_at = updated_at.or_else(|| now_rfc3339_from_metadata(&metadata));
    let id = stable_session_id(&app, &input.path, preview_hash.as_deref());

    Ok((
        AgentSession {
            id,
            app,
            title,
            cwd,
            started_at,
            updated_at,
            turns: Some(stats.known_events),
            tokens: (token_usage.input.is_some()
                || token_usage.output.is_some()
                || token_usage.total.is_some())
            .then_some(token_usage),
            files: Vec::new(),
            size_bytes: metadata.len(),
            preview,
            preview_hash,
            risk: RiskLevel::Yellow,
            source: SessionSource::File(input.path.clone()),
            parser: "jsonl-session".to_owned(),
        },
        stats,
    ))
}

fn update_bounds(
    timestamp: &str,
    started_at: &mut Option<String>,
    updated_at: &mut Option<String>,
) {
    let Ok(parsed) = DateTime::parse_from_rfc3339(timestamp) else {
        return;
    };
    let normalized = parsed.with_timezone(&Utc).to_rfc3339();
    if started_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_none_or(|current| parsed < current)
    {
        *started_at = Some(normalized.clone());
    }
    if updated_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_none_or(|current| parsed > current)
    {
        *updated_at = Some(normalized);
    }
}

fn merge_usage(tokens: &mut TokenUsage, usage: UsageEvent) {
    tokens.input = merge_counter(tokens.input, usage.input);
    tokens.output = merge_counter(tokens.output, usage.output);
    tokens.total = merge_counter(tokens.total, usage.total);
}

fn merge_counter(current: Option<u64>, next: Option<u64>) -> Option<u64> {
    match (current, next) {
        (Some(current), Some(next)) => Some(current.saturating_add(next)),
        (Some(current), None) => Some(current),
        (None, Some(next)) => Some(next),
        (None, None) => None,
    }
}

fn content_to_string(value: Option<&Value>) -> Option<String> {
    let value = value?;
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Array(values) => {
            let parts = values
                .iter()
                .filter_map(|value| content_to_string(Some(value)))
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join(" "))
        }
        Value::Object(map) => map
            .get("text")
            .or_else(|| map.get("content"))
            .and_then(|value| content_to_string(Some(value))),
        _ => None,
    }
}

fn first_words(value: &str, max_words: usize) -> String {
    let words = value.split_whitespace().take(max_words).collect::<Vec<_>>();
    if words.is_empty() {
        "Untitled session".to_owned()
    } else {
        words.join(" ")
    }
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

fn stable_session_id(app: &AppId, path: &std::path::Path, preview_hash: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(app.key().as_bytes());
    hasher.update(path.to_string_lossy().as_bytes());
    if let Some(preview_hash) = preview_hash {
        hasher.update(preview_hash.as_bytes());
    }
    format!("session_{}", &hex::encode(hasher.finalize())[..16])
}

fn now_rfc3339_from_metadata(metadata: &std::fs::Metadata) -> Option<String> {
    metadata
        .modified()
        .ok()
        .map(chrono::DateTime::<Utc>::from)
        .map(|time| time.to_rfc3339())
        .or_else(|| Some(now_rfc3339()))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::parse_jsonl_session;
    use crate::ParserInput;

    #[test]
    fn parses_known_and_unknown_events() {
        let mut file = NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"type":"user","timestamp":"2026-05-15T10:00:00Z","cwd":"/work/demo","content":"Write a README"}}"#
        )
        .expect("write user");
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-15T10:01:00Z","cwd":"/work/demo","content":"Here is the README..."}}"#
        )
        .expect("write assistant");
        writeln!(
            file,
            r#"{{"type":"unknown_event","timestamp":"2026-05-15T10:02:00Z","payload":{{}}}}"#
        )
        .expect("write unknown");

        let (session, stats) = parse_jsonl_session(
            &ParserInput {
                path: file.path().to_path_buf(),
                app_hint: Some("claude".to_owned()),
            },
            &vibe_hauler_redaction::RedactionPolicy::default(),
        )
        .expect("jsonl parses");

        assert_eq!(stats.known_events, 2);
        assert_eq!(stats.unknown_events, 1);
        assert_eq!(session.title.as_deref(), Some("Write a README"));
        assert_eq!(session.turns, Some(2));
    }
}

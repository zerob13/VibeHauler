use std::{fs, path::Path};

use anyhow::Context;
use sha2::{Digest, Sha256};
use vibe_hauler_core::{AgentSession, AppId, RiskLevel, SessionSource, now_rfc3339};
use vibe_hauler_redaction::{RedactionPolicy, redact_text};

use crate::{ParserInput, ParserSupport, SessionParser};

#[derive(Clone, Copy, Debug, Default)]
pub struct MarkdownHistoryParser;

impl SessionParser for MarkdownHistoryParser {
    fn name(&self) -> &'static str {
        "aider-markdown-history"
    }

    fn supports(&self, input: &ParserInput) -> ParserSupport {
        let Some(name) = input.path.file_name().and_then(|name| name.to_str()) else {
            return ParserSupport::Unsupported;
        };
        if matches!(
            name,
            ".aider.chat.history.md" | ".aider.input.history" | ".aider.llm.history"
        ) {
            ParserSupport::Supported
        } else if input.path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            ParserSupport::Maybe
        } else {
            ParserSupport::Unsupported
        }
    }

    fn parse(&self, input: ParserInput) -> anyhow::Result<Vec<AgentSession>> {
        Ok(vec![parse_aider_history(
            &input,
            &RedactionPolicy::default(),
        )?])
    }
}

pub fn parse_aider_history(
    input: &ParserInput,
    redaction_policy: &RedactionPolicy,
) -> anyhow::Result<AgentSession> {
    let value = fs::read_to_string(&input.path)
        .with_context(|| format!("failed to read markdown history {}", input.path.display()))?;
    let metadata = fs::metadata(&input.path)?;
    let sections = parse_sections(&value);
    let turns = u32::try_from(
        sections
            .iter()
            .filter(|section| matches!(section.role.as_str(), "User" | "Assistant"))
            .count(),
    )
    .unwrap_or(u32::MAX);
    let preview_source = sections
        .iter()
        .find(|section| section.role == "User")
        .map_or_else(|| value.as_str(), |section| section.body.as_str());
    let preview = redact_text(preview_source, redaction_policy).value;
    let title = Some(first_words(preview_source, 8));
    let preview_hash = Some(hash_text(&preview));
    let app = input
        .app_hint
        .as_deref()
        .map_or(AppId::Aider, AppId::from_key);
    let cwd = input.path.parent().map(Path::to_path_buf);
    let updated_at = metadata
        .modified()
        .ok()
        .map(chrono::DateTime::<chrono::Utc>::from)
        .map(|time| time.to_rfc3339())
        .or_else(|| Some(now_rfc3339()));

    Ok(AgentSession {
        id: stable_session_id(&app, &input.path, preview_hash.as_deref()),
        app,
        title,
        cwd,
        started_at: updated_at.clone(),
        updated_at,
        turns: Some(turns),
        tokens: None,
        files: Vec::new(),
        size_bytes: metadata.len(),
        preview: Some(preview),
        preview_hash,
        risk: RiskLevel::Yellow,
        source: SessionSource::File(input.path.clone()),
        parser: "aider-markdown-history".to_owned(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Section {
    role: String,
    body: String,
}

fn parse_sections(value: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut role = String::new();
    let mut body = Vec::new();

    for line in value.lines() {
        if let Some(next_role) = role_heading(line) {
            if !role.is_empty() {
                sections.push(Section {
                    role: role.clone(),
                    body: body.join("\n").trim().to_owned(),
                });
                body.clear();
            }
            next_role.clone_into(&mut role);
        } else if !role.is_empty() {
            body.push(line.to_owned());
        }
    }

    if !role.is_empty() {
        sections.push(Section {
            role,
            body: body.join("\n").trim().to_owned(),
        });
    }

    sections
}

fn role_heading(line: &str) -> Option<&'static str> {
    let normalized = line.trim().trim_start_matches('#').trim();
    if normalized.eq_ignore_ascii_case("user") {
        Some("User")
    } else if normalized.eq_ignore_ascii_case("assistant") {
        Some("Assistant")
    } else if normalized.eq_ignore_ascii_case("system") {
        Some("System")
    } else {
        None
    }
}

fn first_words(value: &str, max_words: usize) -> String {
    let words = value.split_whitespace().take(max_words).collect::<Vec<_>>();
    if words.is_empty() {
        "Aider history".to_owned()
    } else {
        words.join(" ")
    }
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

fn stable_session_id(app: &AppId, path: &Path, preview_hash: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(app.key().as_bytes());
    hasher.update(path.to_string_lossy().as_bytes());
    if let Some(preview_hash) = preview_hash {
        hasher.update(preview_hash.as_bytes());
    }
    format!("session_{}", &hex::encode(hasher.finalize())[..16])
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::parse_aider_history;
    use crate::ParserInput;

    #[test]
    fn parses_aider_markdown_format() {
        let mut file = NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            "# Aider Chat History\n\n## User\nAdd docstrings to all functions\n\n## Assistant\nI'll add docstrings."
        )
        .expect("write markdown");

        let session = parse_aider_history(
            &ParserInput {
                path: file.path().to_path_buf(),
                app_hint: Some("aider".to_owned()),
            },
            &vibe_hauler_redaction::RedactionPolicy::default(),
        )
        .expect("markdown parses");

        assert_eq!(session.turns, Some(2));
        assert_eq!(
            session.title.as_deref(),
            Some("Add docstrings to all functions")
        );
    }
}

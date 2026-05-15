use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{SecondsFormat, TimeZone, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use sha2::{Digest, Sha256};
use vibe_hauler_core::{AgentSession, AppId, RiskLevel, SessionSource, TokenUsage};
use vibe_hauler_redaction::{RedactionPolicy, redact_text};

use crate::{ParserInput, ParserSupport, SessionParser};

#[derive(Clone, Copy, Debug, Default)]
pub struct SqliteIntrospector;

#[derive(Clone, Copy, Debug, Default)]
pub struct DeepChatSqliteParser;

#[derive(Clone, Debug)]
struct SessionSummary {
    source_id: String,
    title: Option<String>,
    cwd: Option<PathBuf>,
    started_at: Option<String>,
    updated_at: Option<String>,
    turns: u32,
    tokens: Option<TokenUsage>,
    preview_parts: Vec<String>,
}

#[derive(Clone, Debug)]
struct DeepChatMessage {
    role: String,
    content: String,
}

impl SessionParser for DeepChatSqliteParser {
    fn name(&self) -> &'static str {
        "deepchat-sqlite"
    }

    fn supports(&self, input: &ParserInput) -> ParserSupport {
        let is_deepchat = input
            .app_hint
            .as_deref()
            .is_some_and(|value| AppId::from_key(value) == AppId::DeepChat);
        let is_deepchat_db = input
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| matches!(name, "agent.db" | "chat.db"));

        if is_deepchat && is_deepchat_db {
            ParserSupport::Supported
        } else if input.path.extension().and_then(|ext| ext.to_str()) == Some("db") {
            ParserSupport::Maybe
        } else {
            ParserSupport::Unsupported
        }
    }

    fn parse(&self, input: ParserInput) -> anyhow::Result<Vec<AgentSession>> {
        parse_deepchat_sessions(&input, &RedactionPolicy::default())
    }
}

pub fn parse_deepchat_sessions(
    input: &ParserInput,
    redaction_policy: &RedactionPolicy,
) -> anyhow::Result<Vec<AgentSession>> {
    let connection = open_readonly(&input.path)?;
    let metadata = std::fs::metadata(&input.path)
        .with_context(|| format!("failed to stat SQLite DB {}", input.path.display()))?;
    let summaries = if has_tables(&connection, &["new_sessions", "deepchat_messages"])? {
        parse_agent_deepchat(&connection)?
    } else if has_tables(&connection, &["deepchat_sessions", "deepchat_messages"])? {
        parse_current_deepchat(&connection)?
    } else if has_tables(&connection, &["conversations", "messages"])? {
        parse_legacy_deepchat(&connection)?
    } else {
        Vec::new()
    };

    Ok(summaries
        .into_iter()
        .map(|summary| build_session(summary, input, metadata.len(), redaction_policy))
        .collect())
}

fn open_readonly(path: &Path) -> anyhow::Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open SQLite DB read-only: {}", path.display()))
}

fn has_tables(connection: &Connection, names: &[&str]) -> anyhow::Result<bool> {
    for name in names {
        if !table_exists(connection, name)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn table_exists(connection: &Connection, name: &str) -> anyhow::Result<bool> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            params![name],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(exists)
}

fn parse_agent_deepchat(connection: &Connection) -> anyhow::Result<Vec<SessionSummary>> {
    let has_usage = table_exists(connection, "deepchat_usage_stats")?;
    let query = if has_usage {
        AGENT_SESSION_QUERY_WITH_USAGE
    } else {
        AGENT_SESSION_QUERY
    };
    let mut statement = connection.prepare(query)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>("id")?,
            row.get::<_, Option<String>>("title")?,
            row.get::<_, Option<String>>("project_dir")?,
            row.get::<_, Option<i64>>("created_at")?,
            row.get::<_, Option<i64>>("updated_at")?,
            row.get::<_, i64>("turns")?,
            row.get::<_, Option<i64>>("input_tokens")?,
            row.get::<_, Option<i64>>("output_tokens")?,
            row.get::<_, Option<i64>>("total_tokens")?,
        ))
    })?;

    rows.map(|row| {
        let (
            source_id,
            title,
            project_dir,
            created_at,
            updated_at,
            turns,
            input_tokens,
            output_tokens,
            total_tokens,
        ) = row?;
        let messages = current_preview_messages(connection, &source_id)?;
        let title = normalized_title(title).or_else(|| first_user_title(&messages));
        Ok(SessionSummary {
            source_id,
            title,
            cwd: project_dir.and_then(|value| {
                let value = value.trim();
                (!value.is_empty()).then(|| PathBuf::from(value))
            }),
            started_at: created_at.and_then(timestamp_to_rfc3339),
            updated_at: updated_at.and_then(timestamp_to_rfc3339),
            turns: u32::try_from(turns).unwrap_or(u32::MAX),
            tokens: usage_from_sums(input_tokens, output_tokens, total_tokens),
            preview_parts: format_preview_parts(messages),
        })
    })
    .collect()
}

fn parse_current_deepchat(connection: &Connection) -> anyhow::Result<Vec<SessionSummary>> {
    let has_usage = table_exists(connection, "deepchat_usage_stats")?;
    let query = if has_usage {
        CURRENT_SESSION_QUERY_WITH_USAGE
    } else {
        CURRENT_SESSION_QUERY
    };
    let mut statement = connection.prepare(query)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>("id")?,
            row.get::<_, Option<String>>("summary_text")?,
            row.get::<_, Option<i64>>("started_at")?,
            row.get::<_, Option<i64>>("updated_at")?,
            row.get::<_, i64>("turns")?,
            row.get::<_, Option<i64>>("input_tokens")?,
            row.get::<_, Option<i64>>("output_tokens")?,
            row.get::<_, Option<i64>>("total_tokens")?,
        ))
    })?;

    rows.map(|row| {
        let (
            source_id,
            title,
            started_at,
            updated_at,
            turns,
            input_tokens,
            output_tokens,
            total_tokens,
        ) = row?;
        let messages = current_preview_messages(connection, &source_id)?;
        let title = normalized_title(title).or_else(|| first_user_title(&messages));
        Ok(SessionSummary {
            source_id,
            title,
            cwd: None,
            started_at: started_at.and_then(timestamp_to_rfc3339),
            updated_at: updated_at.and_then(timestamp_to_rfc3339),
            turns: u32::try_from(turns).unwrap_or(u32::MAX),
            tokens: usage_from_sums(input_tokens, output_tokens, total_tokens),
            preview_parts: format_preview_parts(messages),
        })
    })
    .collect()
}

fn current_preview_messages(
    connection: &Connection,
    session_id: &str,
) -> anyhow::Result<Vec<DeepChatMessage>> {
    let has_user_messages = table_exists(connection, "deepchat_user_messages")?;
    let query = if has_user_messages {
        CURRENT_PREVIEW_QUERY_WITH_USER_TEXT
    } else {
        CURRENT_PREVIEW_QUERY
    };
    let mut statement = connection.prepare(query)?;
    let rows = statement.query_map(params![session_id], |row| {
        Ok((
            row.get::<_, String>("id")?,
            row.get::<_, String>("role")?,
            row.get::<_, String>("content")?,
            row.get::<_, Option<String>>("user_text")?,
        ))
    })?;

    rows.map(|row| {
        let (message_id, role, content, user_text) = row?;
        let content = if role == "assistant" {
            assistant_block_text(connection, &message_id)?.unwrap_or(content)
        } else {
            user_text.unwrap_or(content)
        };
        Ok(DeepChatMessage { role, content })
    })
    .collect()
}

fn assistant_block_text(
    connection: &Connection,
    message_id: &str,
) -> anyhow::Result<Option<String>> {
    if !table_exists(connection, "deepchat_assistant_blocks")? {
        return Ok(None);
    }

    let mut statement = connection.prepare(
        "SELECT text_content
         FROM deepchat_assistant_blocks
         WHERE message_id = ?1 AND text_content IS NOT NULL AND text_content != ''
         ORDER BY block_index
         LIMIT 6",
    )?;
    let parts = statement
        .query_map(params![message_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((!parts.is_empty()).then(|| parts.join("\n")))
}

fn parse_legacy_deepchat(connection: &Connection) -> anyhow::Result<Vec<SessionSummary>> {
    let mut statement = connection.prepare(LEGACY_SESSION_QUERY)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>("conv_id")?,
            row.get::<_, Option<String>>("title")?,
            row.get::<_, Option<i64>>("created_at")?,
            row.get::<_, Option<i64>>("updated_at")?,
            row.get::<_, i64>("turns")?,
            row.get::<_, Option<i64>>("total_tokens")?,
        ))
    })?;

    rows.map(|row| {
        let (source_id, title, created_at, updated_at, turns, total_tokens) = row?;
        let messages = legacy_preview_messages(connection, &source_id)?;
        let title = normalized_title(title).or_else(|| first_user_title(&messages));
        Ok(SessionSummary {
            source_id,
            title,
            cwd: None,
            started_at: created_at.and_then(timestamp_to_rfc3339),
            updated_at: updated_at.and_then(timestamp_to_rfc3339),
            turns: u32::try_from(turns).unwrap_or(u32::MAX),
            tokens: usage_from_sums(None, None, total_tokens),
            preview_parts: format_preview_parts(messages),
        })
    })
    .collect()
}

fn legacy_preview_messages(
    connection: &Connection,
    conversation_id: &str,
) -> anyhow::Result<Vec<DeepChatMessage>> {
    let mut statement = connection.prepare(
        "SELECT role, content
         FROM messages
         WHERE conversation_id = ?1
         ORDER BY order_seq, created_at
         LIMIT 8",
    )?;
    let rows = statement.query_map(params![conversation_id], |row| {
        Ok(DeepChatMessage {
            role: row.get(0)?,
            content: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn build_session(
    summary: SessionSummary,
    input: &ParserInput,
    size_bytes: u64,
    redaction_policy: &RedactionPolicy,
) -> AgentSession {
    let raw_preview = if summary.preview_parts.is_empty() {
        summary
            .title
            .clone()
            .unwrap_or_else(|| "No preview available".to_owned())
    } else {
        summary.preview_parts.join("\n")
    };
    let preview = redact_text(&raw_preview, redaction_policy).value;
    let preview_hash = Some(hash_text(&preview));
    let id = stable_session_id(
        &AppId::DeepChat,
        &input.path,
        &summary.source_id,
        preview_hash.as_deref(),
    );

    AgentSession {
        id,
        app: AppId::DeepChat,
        title: summary.title,
        cwd: summary.cwd,
        started_at: summary.started_at,
        updated_at: summary.updated_at,
        turns: Some(summary.turns),
        tokens: summary.tokens,
        files: Vec::new(),
        size_bytes,
        preview: Some(preview),
        preview_hash,
        risk: RiskLevel::Black,
        source: SessionSource::Database(input.path.clone()),
        parser: "deepchat-sqlite".to_owned(),
    }
}

fn usage_from_sums(
    input: Option<i64>,
    output: Option<i64>,
    total: Option<i64>,
) -> Option<TokenUsage> {
    let usage = TokenUsage {
        input: positive_i64_to_u64(input),
        output: positive_i64_to_u64(output),
        total: positive_i64_to_u64(total),
    };
    (usage.input.is_some() || usage.output.is_some() || usage.total.is_some()).then_some(usage)
}

fn positive_i64_to_u64(value: Option<i64>) -> Option<u64> {
    value
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
}

fn timestamp_to_rfc3339(value: i64) -> Option<String> {
    let millis = if value > 10_000_000_000 {
        value
    } else {
        value.checked_mul(1000)?
    };
    Utc.timestamp_millis_opt(millis)
        .single()
        .map(|time| time.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn normalized_title(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| first_words(value, 12))
    })
}

fn first_user_title(messages: &[DeepChatMessage]) -> Option<String> {
    messages
        .iter()
        .find(|message| message.role == "user" && !message.content.trim().is_empty())
        .map(|message| first_words(&message.content, 8))
}

fn format_preview_parts(messages: Vec<DeepChatMessage>) -> Vec<String> {
    messages
        .into_iter()
        .filter_map(|message| {
            let content = message.content.trim();
            (!content.is_empty()).then(|| format!("{}: {content}", message.role))
        })
        .collect()
}

fn first_words(value: &str, max_words: usize) -> String {
    let words = value.split_whitespace().take(max_words).collect::<Vec<_>>();
    if words.is_empty() {
        "Untitled DeepChat session".to_owned()
    } else {
        words.join(" ")
    }
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

fn stable_session_id(
    app: &AppId,
    path: &Path,
    source_id: &str,
    preview_hash: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(app.key().as_bytes());
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(source_id.as_bytes());
    if let Some(preview_hash) = preview_hash {
        hasher.update(preview_hash.as_bytes());
    }
    format!("session_{}", &hex::encode(hasher.finalize())[..16])
}

const CURRENT_SESSION_QUERY: &str = "
    SELECT
        s.id,
        s.summary_text,
        MIN(m.created_at) AS started_at,
        COALESCE(MAX(m.updated_at), s.summary_updated_at) AS updated_at,
        COUNT(m.id) AS turns,
        NULL AS input_tokens,
        NULL AS output_tokens,
        NULL AS total_tokens
    FROM deepchat_sessions s
    LEFT JOIN deepchat_messages m ON m.session_id = s.id
    GROUP BY s.id
    ORDER BY COALESCE(MAX(m.updated_at), s.summary_updated_at, 0) DESC, s.id";

const CURRENT_SESSION_QUERY_WITH_USAGE: &str = "
    SELECT
        s.id,
        s.summary_text,
        MIN(m.created_at) AS started_at,
        COALESCE(MAX(m.updated_at), s.summary_updated_at) AS updated_at,
        COUNT(DISTINCT m.id) AS turns,
        SUM(u.input_tokens) AS input_tokens,
        SUM(u.output_tokens) AS output_tokens,
        SUM(u.total_tokens) AS total_tokens
    FROM deepchat_sessions s
    LEFT JOIN deepchat_messages m ON m.session_id = s.id
    LEFT JOIN deepchat_usage_stats u ON u.message_id = m.id
    GROUP BY s.id
    ORDER BY COALESCE(MAX(m.updated_at), s.summary_updated_at, 0) DESC, s.id";

const AGENT_SESSION_QUERY: &str = "
    SELECT
        s.id,
        s.title,
        s.project_dir,
        s.created_at,
        s.updated_at,
        COUNT(m.id) AS turns,
        NULL AS input_tokens,
        NULL AS output_tokens,
        NULL AS total_tokens
    FROM new_sessions s
    LEFT JOIN deepchat_messages m ON m.session_id = s.id
    GROUP BY s.id
    ORDER BY s.updated_at DESC, s.id";

const AGENT_SESSION_QUERY_WITH_USAGE: &str = "
    SELECT
        s.id,
        s.title,
        s.project_dir,
        s.created_at,
        s.updated_at,
        COUNT(DISTINCT m.id) AS turns,
        SUM(u.input_tokens) AS input_tokens,
        SUM(u.output_tokens) AS output_tokens,
        SUM(u.total_tokens) AS total_tokens
    FROM new_sessions s
    LEFT JOIN deepchat_messages m ON m.session_id = s.id
    LEFT JOIN deepchat_usage_stats u ON u.message_id = m.id
    GROUP BY s.id
    ORDER BY s.updated_at DESC, s.id";

const CURRENT_PREVIEW_QUERY: &str = "
    SELECT id, role, content, NULL AS user_text
    FROM deepchat_messages
    WHERE session_id = ?1
    ORDER BY order_seq, created_at
    LIMIT 8";

const CURRENT_PREVIEW_QUERY_WITH_USER_TEXT: &str = "
    SELECT m.id, m.role, m.content, u.text AS user_text
    FROM deepchat_messages m
    LEFT JOIN deepchat_user_messages u ON u.message_id = m.id
    WHERE m.session_id = ?1
    ORDER BY m.order_seq, m.created_at
    LIMIT 8";

const LEGACY_SESSION_QUERY: &str = "
    SELECT
        c.conv_id,
        c.title,
        COALESCE(MIN(m.created_at), c.created_at) AS created_at,
        COALESCE(MAX(m.created_at), c.updated_at) AS updated_at,
        COUNT(m.msg_id) AS turns,
        SUM(m.token_count) AS total_tokens
    FROM conversations c
    LEFT JOIN messages m ON m.conversation_id = c.conv_id
    GROUP BY c.conv_id
    ORDER BY COALESCE(MAX(m.created_at), c.updated_at, 0) DESC, c.conv_id";

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    use super::parse_deepchat_sessions;
    use crate::ParserInput;

    #[test]
    fn parses_agent_deepchat_schema() {
        let file = NamedTempFile::new().expect("temp db");
        let connection = Connection::open(file.path()).expect("open db");
        connection
            .execute_batch(AGENT_DB_FIXTURE_SQL)
            .expect("seed db");
        drop(connection);

        let sessions = parse_deepchat_sessions(
            &ParserInput {
                path: file.path().to_path_buf(),
                app_hint: Some("deepchat".to_owned()),
            },
            &vibe_hauler_redaction::RedactionPolicy::default(),
        )
        .expect("parse sessions");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title.as_deref(), Some("README summary"));
        assert_eq!(
            sessions[0].cwd.as_deref(),
            Some(std::path::Path::new("/work/demo"))
        );
        assert_eq!(sessions[0].turns, Some(2));
        assert_eq!(
            sessions[0].preview.as_deref(),
            Some("user: Write a README for VibeHauler\nassistant: Here is the README...")
        );
        assert_eq!(
            sessions[0].tokens.as_ref().and_then(|usage| usage.total),
            Some(30)
        );
        assert_eq!(sessions[0].risk, vibe_hauler_core::RiskLevel::Black);
    }

    const AGENT_DB_FIXTURE_SQL: &str = "
        CREATE TABLE new_sessions (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            title TEXT NOT NULL,
            project_dir TEXT,
            is_pinned INTEGER DEFAULT 0,
            is_draft INTEGER NOT NULL DEFAULT 0,
            active_skills TEXT NOT NULL DEFAULT '[]',
            disabled_agent_tools TEXT NOT NULL DEFAULT '[]',
            subagent_enabled INTEGER NOT NULL DEFAULT 0,
            session_kind TEXT NOT NULL DEFAULT 'regular',
            parent_session_id TEXT,
            subagent_meta_json TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE deepchat_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            order_seq INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            status TEXT DEFAULT 'sent',
            is_context_edge INTEGER DEFAULT 0,
            metadata TEXT DEFAULT '{}',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE deepchat_user_messages (
            message_id TEXT PRIMARY KEY,
            text TEXT NOT NULL DEFAULT '',
            search_enabled INTEGER NOT NULL DEFAULT 0,
            think_enabled INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE deepchat_assistant_blocks (
            message_id TEXT NOT NULL,
            block_index INTEGER NOT NULL,
            block_type TEXT NOT NULL,
            status TEXT NOT NULL,
            text_content TEXT,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (message_id, block_index)
        );
        CREATE TABLE deepchat_usage_stats (
            message_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            usage_date TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            model_id TEXT NOT NULL,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            cached_input_tokens INTEGER NOT NULL DEFAULT 0,
            estimated_cost_usd REAL,
            source TEXT NOT NULL DEFAULT 'live',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        INSERT INTO new_sessions
            (id, agent_id, title, project_dir, created_at, updated_at)
            VALUES ('s1', 'deepchat', 'README summary', '/work/demo',
                    1770000000000, 1770000060000);
        INSERT INTO deepchat_messages
            (id, session_id, order_seq, role, content, status, created_at, updated_at)
            VALUES
            ('m1', 's1', 1, 'user', '', 'sent', 1770000000000, 1770000000000),
            ('m2', 's1', 2, 'assistant', '', 'sent', 1770000060000, 1770000060000);
        INSERT INTO deepchat_user_messages
            (message_id, text, search_enabled, think_enabled)
            VALUES ('m1', 'Write a README for VibeHauler', 0, 0);
        INSERT INTO deepchat_assistant_blocks
            (message_id, block_index, block_type, status, text_content, updated_at)
            VALUES ('m2', 0, 'content', 'success', 'Here is the README...', 1770000060000);
        INSERT INTO deepchat_usage_stats
            (message_id, session_id, usage_date, provider_id, model_id, input_tokens,
             output_tokens, total_tokens, cached_input_tokens, source, created_at, updated_at)
            VALUES ('m2', 's1', '2026-02-02', 'openai', 'gpt-4.1', 10, 20, 30, 0, 'live',
                    1770000060000, 1770000060000);
    ";

    #[test]
    fn parses_legacy_deepchat_schema() {
        let file = NamedTempFile::new().expect("temp db");
        let connection = Connection::open(file.path()).expect("open db");
        connection
            .execute_batch(
                "
                CREATE TABLE conversations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    conv_id TEXT UNIQUE NOT NULL,
                    title TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    msg_id TEXT UNIQUE NOT NULL,
                    conversation_id TEXT NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    order_seq INTEGER NOT NULL DEFAULT 0,
                    token_count INTEGER DEFAULT 0
                );
                INSERT INTO conversations
                    (conv_id, title, created_at, updated_at)
                    VALUES ('c1', 'Legacy chat', 1770000000000, 1770000060000);
                INSERT INTO messages
                    (msg_id, conversation_id, role, content, created_at, order_seq, token_count)
                    VALUES
                    ('lm1', 'c1', 'user', 'Explain MCP', 1770000000000, 1, 4),
                    ('lm2', 'c1', 'assistant', 'MCP connects tools.', 1770000060000, 2, 6);
                ",
            )
            .expect("seed db");
        drop(connection);

        let sessions = parse_deepchat_sessions(
            &ParserInput {
                path: file.path().to_path_buf(),
                app_hint: Some("deepchat".to_owned()),
            },
            &vibe_hauler_redaction::RedactionPolicy::default(),
        )
        .expect("parse sessions");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title.as_deref(), Some("Legacy chat"));
        assert_eq!(sessions[0].turns, Some(2));
        assert_eq!(
            sessions[0].tokens.as_ref().and_then(|usage| usage.total),
            Some(10)
        );
    }
}

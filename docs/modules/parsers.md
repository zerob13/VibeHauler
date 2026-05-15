# Parsers Module

Cargo package: `vibe-hauler-parsers`

## Responsibility

Parsers understand file and database formats. They should be reusable across adapters and must degrade gracefully when schemas drift.

## Parser Families

| Parser | Targets |
|---|---|
| JSONL session parser | Claude, Codex, Gemini, OpenCode |
| Markdown history parser | Aider |
| SQLite introspector | Cursor, DeepChat, Alma, Goose, Cherry v2 |
| Chromium localStorage LevelDB parser | Cherry, Electron apps |
| IndexedDB raw scanner | Cherry Dexie stores |
| Generic file sniffer | Droid, OpenCode, unknown agents |

## Trait

```rust
pub trait SessionParser: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, input: &ParserInput) -> ParserSupport;
    fn parse(&self, input: ParserInput) -> anyhow::Result<Vec<AgentSession>>;
}
```

## JSONL Parser

Requirements:

- parse line by line;
- tolerate unknown event types;
- extract timestamps, cwd, session id, role, content, summary, usage;
- preserve warnings without failing the whole file;
- derive title from summary or first user message.

## Markdown Parser

Requirements:

- segment Aider history by headings and role markers;
- infer repo root from file location;
- support `.aider.chat.history.md`, `.aider.input.history`, and `.aider.llm.history`;
- treat input history as sensitive Yellow, not cache.

## SQLite Introspector

Requirements:

- open read-only;
- use schema introspection first;
- identify common KV tables and chat tables;
- row count and rough table size if available;
- never write `VACUUM`, `ANALYZE`, or temp artifacts to the source DB.

## LevelDB Parser

Requirements:

- open read-only;
- decode Chromium localStorage strings;
- tolerate UTF-16LE, NUL-stripping, and nested JSON strings;
- if locked, return Black evidence instead of retrying destructively.

## IndexedDB Scanner

Requirements:

- scan `.log`, `.ldb`, and `.sst` files for candidate serialized values;
- bounded parsing with max depth, max slice size, max objects;
- detect topic/message block shapes;
- never rewrite LevelDB files.

## Testing

- Fixture files for every parser.
- Golden JSON outputs.
- Malformed input tests.
- Large-file streaming test for JSONL.
- Locked DB behavior for SQLite/LevelDB.


# Adapter Plan

Adapters are the app-specific layer. Their job is to turn local client data into inventory, sessions, and evidence without making irreversible decisions.

## Support Matrix

| App | Stage | Tier | Root Discovery | Session Source | Cleanup Scope |
|---|---|---|---|---|---|
| Claude Code | v0.1 | Full | `CLAUDE_CONFIG_DIR`, `~/.claude` | `projects/**/*.jsonl` | logs/cache/tmp; sessions only by selection |
| Codex | v0.1 | Full | `CODEX_HOME`, `~/.codex` | `history.jsonl`, `sessions/**` | logs/cache/tmp; sessions only by selection |
| Gemini CLI | v0.1 | Full | `~/.gemini` | `tmp/*/chats/**` | logs/cache/tmp; chats only by selection |
| Aider | v0.1 | Full | repo-local | `.aider.chat.history.md` | history only by selection |
| OpenCode | v0.2 | Sniffer | XDG config/data/cache | JSON/JSONL sniff | logs/cache/tmp |
| Cursor | v0.2 | Protective | VS Code-like user data | `state.vscdb` key families | cache/log only |
| Goose | v0.2 | Protective | data/config dirs | `sessions.db` report | logs only |
| Cherry Studio | v0.3 | Protective | Electron userData | backup JSON, localStorage report | trace/cache/log only |
| DeepChat | v0.3 | Protective | Electron userData | SQLite schema report | cache/log only |
| Alma | v0.3 | Protective | Electron userData | `chat_threads.db` report | none by default |
| Factory Droid | later | Sniffer | `~/.factory`, repo `.factory` | file fingerprinting | logs/tmp only |
| Copilot CLI | later | Sniffer | `~/.copilot` | logs/cache initially | logs/cache |

## Shared Adapter Rules

1. Always protect auth/config/memory/rules/skills/commands.
2. Prefer readonly database access.
3. If a DB or LevelDB store is locked, mark Black.
4. If schema is unknown, emit a schema report instead of failing.
5. Session cleanup is Yellow by default.
6. App-specific “safe cleanup” should be limited to clearly rebuildable logs/cache/tmp.

## Claude Code

Roots:

- `CLAUDE_CONFIG_DIR`
- `~/.claude`
- Windows `%USERPROFILE%\.claude`

Inventory:

| Path | Risk | Action |
|---|---|---|
| `projects/**/*.jsonl` | Yellow | parse sessions; selected backup+trash |
| `settings*.json` | Red | protect |
| `CLAUDE.md`, memory, skills, commands, agents | Red | protect |
| logs/cache/tmp | Green | safe cleanup |

Parser:

- JSONL event scan.
- Derive cwd from event or encoded project path.
- Title from summary or first user message.

## Codex

Roots:

- `CODEX_HOME`
- `~/.codex`
- Windows `%USERPROFILE%\.codex`

Inventory:

| Path | Risk | Action |
|---|---|---|
| `history.jsonl` | Yellow | session/history view |
| `sessions/**`, `archived_sessions/**` | Yellow | selected backup+trash |
| `config.toml`, `auth.json` | Red | protect |
| logs/cache/tmp | Green | safe cleanup |

Parser:

- JSONL parser.
- TOML metadata where present.
- Directory date fallback.

## Gemini CLI

Root:

- `~/.gemini`

Inventory:

| Path | Risk | Action |
|---|---|---|
| `settings.json` | Red | protect |
| `tmp/<project_hash>/chats/**` | Yellow | parse sessions |
| logs/cache/tmp | Green | safe cleanup |

Parser:

- JSON/JSONL and directory metadata.
- Project hash mapping is best-effort unless user provides roots.

## Aider

Root model:

- repo-local, not global.

Files:

| Path | Risk | Action |
|---|---|---|
| `.aider.chat.history.md` | Yellow | parse sessions |
| `.aider.input.history` | Yellow | input history |
| `.aider.llm.history` | Yellow | raw LLM history |
| `.aider.conf.yml` | Red | protect |

Parser:

- Markdown segmentation.
- Repo root from file location.

## Cursor

Roots:

- macOS `~/Library/Application Support/Cursor/User`
- Linux `~/.config/Cursor/User`
- Windows `%APPDATA%\Cursor\User`

Inventory:

| Path | Risk | Action |
|---|---|---|
| `globalStorage/state.vscdb` | Red/Black | readonly key-family report |
| `workspaceStorage/*/state.vscdb` | Yellow/Red | workspace size report |
| Cache/logs/GPUCache | Green | safe cleanup |
| settings/keybindings/extensions state | Red | protect |

Cursor is not part of v0.1. When added, it must not compact or replace Cursor DBs. It may generate a readonly report.

## Cherry Studio

Roots:

- macOS `~/Library/Application Support/CherryStudio`
- Linux `~/.config/CherryStudio` or `~/.config/cherry-studio`
- Windows `%APPDATA%\CherryStudio`

Known structures:

- `Local Storage/leveldb/`: redux-persist state, providers, assistants.
- `IndexedDB/file__0.indexeddb.leveldb/`: Dexie topics and message blocks.
- `Data/agents.db`: newer SQLite path.
- trace/cache/log folders.

Inventory:

| Path | Risk | Action |
|---|---|---|
| trace/cache/log | Green | safe cleanup |
| `Local Storage/leveldb` | Red/Black | readonly parse/report |
| `IndexedDB/*.leveldb` | Red/Black | readonly report only |
| `Data/agents.db` | Red | SQLite report |
| user backup JSON | Yellow | session browser input |

Parser priority:

1. user-provided backup JSON;
2. localStorage LevelDB metadata report;
3. IndexedDB candidate report;
4. SQLite introspection.

## DeepChat

Roots:

- Electron userData.

Inventory:

| Path | Risk | Action |
|---|---|---|
| `app_db/chat.db` | Red | readonly conversation report |
| `app_db/agent.db` | Red | readonly agent report |
| DuckDB knowledge bases | Red | protect |
| cache/logs | Green | safe cleanup |

## Goose

Roots:

- Unix `~/.local/share/goose`, `~/.config/goose`
- Windows `%APPDATA%\Block\goose\data`

Inventory:

| Path | Risk | Action |
|---|---|---|
| `sessions/sessions.db` | Yellow/Red | SQLite report |
| logs/cache | Green | safe cleanup |
| config/history | Red | protect |

## Alma

Roots:

- macOS `~/Library/Application Support/alma`
- Linux `~/.config/alma`, `~/.local/share/alma`
- Windows `%APPDATA%\alma`

Inventory:

| Path | Risk | Action |
|---|---|---|
| `chat_threads.db` | Red | readonly session report |
| provider DB/config | Red | protect |
| vector/memory/embedding stores | Red | protect |
| logs/cache | Green | safe cleanup if clearly separate |

## Factory Droid

Roots:

- `~/.factory`
- repo `.factory`

Inventory:

| Path | Risk | Action |
|---|---|---|
| settings/custom models/hooks | Red | protect |
| memories | Red | protect |
| logs/tmp/session candidates | Yellow | sniff/report |

## Adapter Implementation Order

1. Claude.
2. Codex.
3. Gemini.
4. Aider.
5. OpenCode sniffer.
6. Cursor protective report.
7. Goose SQLite report.
8. Cherry protective report plus backup JSON parser.
9. DeepChat/Alma SQLite reports.
10. Droid/Copilot sniffers.

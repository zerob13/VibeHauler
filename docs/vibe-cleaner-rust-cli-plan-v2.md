# Historical Plan: Vibe Cleaner / vclean

> Deprecated historical reference. Do not treat this file as an implementation contract.
> Current contracts live in [VibeHauler Documentation](./README.md), especially the product spec, CLI spec, data model, and module docs.

> This document is historical reference. The current product name is **VibeHauler**, the CLI binary is **`vhaul`**, and the Rust package/repo name is **`vibe-hauler`**.
>
> Start from [VibeHauler Documentation](./README.md) for the current module docs, architecture, CLI spec, and release plan.

# Vibe Cleaner：Agent 时代的本地数据清理 CLI（Rust）技术方案与当晚实施方案

版本：v0.2  
日期：2026-05-15  
目标形态：Rust CLI，本地优先、内置解析器优先、跨 macOS / Linux / Windows，默认 dry-run，支持多 Agent 客户端的数据扫描、session 解析、清理建议、备份与回滚。

---

## 0. 关键调整

本版方案把运行时架构收敛为 **filesystem-first + built-in parser**：工具直接读取本地文件、SQLite、LevelDB、JSONL、Markdown、Electron userData、VS Code storage，不把上游 SDK 作为运行时依赖。

核心决策：

| 主题 | 方案 |
|---|---|
| SDK 耦合 | 零 SDK 运行依赖；SDK、CLI、源码只作为研究资料和 fixture 来源 |
| 解析方式 | 每个 app adapter 内置文件发现、schema introspection、宽松 parser、格式 fingerprint |
| 跨平台 | 统一 `PathResolver`，覆盖 macOS / Linux / Windows / WSL 候选路径 |
| 删除语义 | 默认系统 Trash；失败时进入 vclean quarantine；永久删除需要显式危险参数 |
| 数据库 | 默认只读；VACUUM / shrink 进入高级动作，先备份再替换 |
| 高风险数据 | credentials、config、memory、rules、skills、MCP auth、keychain 引用默认保护 |
| Session 清理 | 以时间、项目目录、标题、体积、工具、风险等级建立多维索引 |

---

## 1. 产品定位

Vibe Cleaner 是面向 AI coding agent 重度用户的本地清理工具。它扫描 Claude Code、Codex、OpenCode、Cursor、Cherry Studio、DeepChat、Factory Droid、Gemini CLI、Copilot CLI、Goose、Aider、ALMA 等工具的数据目录，分析缓存、日志、临时文件、session transcript、checkpoint、workspace storage、memory store，并输出可解释的清理建议。

产品目标：

- 像 CleanMyMac 一样给出“放心清理 / 谨慎清理 / 默认保留”的分级建议。
- 像 session browser 一样解析 AI agent 的历史上下文，支持按时间、项目、标题、体积、工具筛选。
- 像安全迁移工具一样执行备份、trash、manifest、回滚、SQLite 副本校验。
- 默认本地运行，默认 dry-run，默认只读数据库，默认遮盖敏感内容。

---

## 2. 首晚 MVP 验收目标

命令名暂定：`vclean`。

### 2.1 必须完成

1. `vclean scan`：扫描支持工具的数据目录，输出体积、风险等级、清理候选。
2. `vclean sessions list`：解析 Claude Code、Codex、Gemini CLI、Aider 的 session；OpenCode 做宽松 JSON/JSONL 探测。
3. `vclean plan`：生成清理计划，包含路径、动作、风险、备份策略、预计释放空间。
4. `vclean clean --dry-run`：展示执行效果，写出计划文件。
5. `vclean clean --execute --safe-only`：执行 Green 项清理，进入系统 Trash，生成 manifest。
6. `vclean restore <manifest>`：从 manifest 回滚已清理文件。
7. Cursor、Cherry Studio、DeepChat、ALMA、Droid 首晚以只读扫描和体积分析为主。

### 2.2 首晚默认保护

| 类别 | 保护原因 |
|---|---|
| `auth.json`、OAuth token、API key、keyring 引用 | 删除会导致登录状态丢失或密钥泄露风险 |
| `settings.json`、`config.toml`、`opencode.json(c)` | 用户配置，清理收益低，误删影响高 |
| `CLAUDE.md`、`AGENTS.md`、rules、skills、commands、hooks | 项目行为配置和长期偏好 |
| memory 文件 / vector store / embeddings | 可重建成本高，语义价值高 |
| Cursor / Cherry / DeepChat 核心 DB | 删除会破坏历史、索引或加载状态 |
| 正在被进程占用的 DB / LevelDB | 文件锁或写入竞争风险高 |

### 2.3 首晚可交付范围

| Adapter | 检测 | 体积统计 | Session 解析 | Green 清理 | Yellow 清理 |
|---|:---:|:---:|:---:|:---:|:---:|
| Claude Code | ✅ | ✅ | ✅ | ✅ | 手动选择 |
| Codex | ✅ | ✅ | ✅ | ✅ | 手动选择 |
| Gemini CLI | ✅ | ✅ | ✅ | ✅ | 手动选择 |
| Aider | ✅ | ✅ | ✅ | ✅ | 手动选择 |
| OpenCode | ✅ | ✅ | 宽松解析 | ✅ | 手动选择 |
| Goose | ✅ | ✅ | SQLite schema sniff | logs only | 手动选择 |
| Cursor | ✅ | ✅ | key family preview | cache only | 高级计划 |
| Cherry Studio | ✅ | ✅ | backup / partial parser | trace/cache | 高级计划 |
| DeepChat | ✅ | ✅ | SQLite schema sniff | cache/log | 高级计划 |
| ALMA | ✅ | ✅ | SQLite only | 无 | 保护 |
| Droid | ✅ | ✅ | 文件指纹探测 | logs/tmp | 保护 |

---

## 3. 运行时原则：内置解析器优先

### 3.1 读取优先级

```txt
Local filesystem
  ├─ JSON / JSONL / Markdown / TOML / YAML
  ├─ SQLite readonly + schema introspection
  ├─ LevelDB readonly + Chromium localStorage decoder
  ├─ IndexedDB raw scanner + V8 subset deserializer
  ├─ Electron userData inventory
  ├─ VS Code / Cursor state.vscdb key-value parser
  └─ User-provided backup JSON
```

### 3.2 SDK 边界

运行时依赖清单保持纯 Rust / 系统库 / SQLite / LevelDB 级别。上游 SDK 的角色限定为：

- 研究 session metadata 的语义字段。
- 生成测试 fixture。
- 对照 parser 结果。
- 记录 schema 变化。

`vclean` 的正式解析路径使用本地文件、数据库和缓存。外部命令调用仅用于 `doctor` 或版本探测，并且默认关闭。

### 3.3 Parser 失败策略

| 场景 | 行为 |
|---|---|
| 未知 JSONL 行类型 | 保留 raw event，继续解析下一行 |
| SQLite schema 变化 | 进入 introspection 模式，列出表、列、体积、候选 key |
| LevelDB 锁定 | 标记 Black，提示关闭目标 App 后重试 |
| IndexedDB 无法完整反序列化 | 输出可恢复的 topic/message block 数量，保留 backup JSON 入口 |
| 时间 / cwd 缺失 | 使用 mtime、目录名、workspace metadata fallback |

---

## 4. 跨平台路径设计

### 4.1 PathResolver

`PathResolver` 统一处理 home、config、data、cache、state、Electron userData、VS Code storage、XDG、Windows roaming/local、WSL 路径。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsKind {
    MacOS,
    Linux,
    Windows,
    Wsl,
}

#[derive(Debug, Clone)]
pub struct PathContext {
    pub os: OsKind,
    pub home: PathBuf,
    pub config_dir: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
    pub state_dir: Option<PathBuf>,
    pub app_data_roaming: Option<PathBuf>,
    pub app_data_local: Option<PathBuf>,
}

pub trait RootResolver {
    fn candidate_roots(&self, app: AppId, ctx: &PathContext) -> Vec<CandidateRoot>;
}
```

### 4.2 OS 目录约定

| 目录类型 | macOS | Linux | Windows |
|---|---|---|---|
| Home | `$HOME` | `$HOME` | `%USERPROFILE%` |
| CLI config | `~/.config/<app>` 或 app 自定义 | `${XDG_CONFIG_HOME:-~/.config}/<app>` | `%USERPROFILE%\.config\<app>` 或 `%APPDATA%\<App>` |
| CLI data | `~/.local/share/<app>` 或 app 自定义 | `${XDG_DATA_HOME:-~/.local/share}/<app>` | `%USERPROFILE%\.local\share\<app>` 或 `%APPDATA%\<App>` |
| CLI cache | `~/.cache/<app>` 或 app 自定义 | `${XDG_CACHE_HOME:-~/.cache}/<app>` | `%LOCALAPPDATA%\<App>\Cache` |
| Electron userData | `~/Library/Application Support/<App>` | `${XDG_CONFIG_HOME:-~/.config}/<App>` | `%APPDATA%\<App>` |
| Electron cache | `~/Library/Caches/<App>` | `${XDG_CACHE_HOME:-~/.cache}/<App>` | `%LOCALAPPDATA%\<App>` |
| VS Code-like storage | `~/Library/Application Support/<App>/User` | `~/.config/<App>/User` | `%APPDATA%\<App>\User` |

### 4.3 WSL 策略

WSL 按 Linux 路径扫描。用户显式传入 `--include-windows-home` 时，补扫 `/mnt/c/Users/<name>` 下的 Windows AppData。扫描结果标记 `platform = windows-from-wsl`，清理动作默认进入 dry-run。

### 4.4 自定义路径

```bash
vclean scan --roots claude=/Volumes/Data/.claude,cursor="D:\\CursorData"
vclean sessions list --app aider --repo ~/work --repo ~/oss
vclean scan --portable-root ./fixtures/home
```

---

## 5. 已知工具目录与跨平台策略

### 5.1 总表

| 工具 | macOS 候选 | Linux 候选 | Windows 候选 | 首版策略 |
|---|---|---|---|---|
| Claude Code | `$CLAUDE_CONFIG_DIR` 或 `~/.claude` | 同 macOS | `%USERPROFILE%\.claude` 或 `CLAUDE_CONFIG_DIR` | JSONL session 解析，保护 config/auth/memory |
| Codex | `$CODEX_HOME` 或 `~/.codex` | 同 macOS | `%USERPROFILE%\.codex` 或 `CODEX_HOME` | JSONL / history 解析，保护 auth/config |
| OpenCode | `~/.local/share/opencode`, `~/.cache/opencode`, `~/.config/opencode` | 同 macOS | `%USERPROFILE%\.local\share\opencode`, `%USERPROFILE%\.config\opencode` | 清 log/cache，session 宽松解析 |
| Cursor | `~/Library/Application Support/Cursor/User` | `~/.config/Cursor/User` | `%APPDATA%\Cursor\User` | 只读扫描 state.vscdb，生成 backup+vacuum 计划 |
| Cherry Studio | `~/Library/Application Support/CherryStudio` | `~/.config/CherryStudio` / `~/.config/cherry-studio` | `%APPDATA%\CherryStudio` | trace/cache 清理，LocalStorage / backup 解析，IndexedDB 子集 parser |
| DeepChat | `~/Library/Application Support/DeepChat` | `~/.config/DeepChat` | `%APPDATA%\DeepChat` | app_db 只读，cache/log 清理 |
| Gemini CLI | `~/.gemini` | `~/.gemini` | `%USERPROFILE%\.gemini` | tmp chats 解析和旧会话清理 |
| Goose | `~/.local/share/goose`, `~/.config/goose` | 同 macOS | `%APPDATA%\Block\goose\data` | sessions.db 只读解析，logs 清理 |
| Aider | repo 内 `.aider.chat.history.md`, `.aider.input.history` | 同 macOS | 同 macOS | Markdown session 解析 |
| ALMA | `~/Library/Application Support/alma` | `~/.config/alma` / `~/.local/share/alma` | `%APPDATA%\alma` | SQLite 只读分析，memory/vector 保护 |
| Factory Droid | `~/.factory`, repo `.factory` | 同 macOS | `%USERPROFILE%\.factory`, repo `.factory` | settings/memory 保护，session 文件指纹探测 |
| Copilot CLI | `~/.copilot` | `~/.copilot` | `%USERPROFILE%\.copilot` | logs/cache 清理，session 后续扩展 |

### 5.2 目录发现算法

1. 读取环境变量：`CLAUDE_CONFIG_DIR`、`CODEX_HOME`、`XDG_*`、`APPDATA`、`LOCALAPPDATA`、`USERPROFILE`。
2. 生成官方 / 观察到的 candidate roots。
3. 对每个 candidate root 做轻量 fingerprint：关键文件、目录名、schema、magic bytes。
4. 根据 confidence 排序：`Exact > Strong > Weak > UserProvided`。
5. 同一 App 发现多个实例时保留多个 instance，例如 portable data、测试 fixture、WSL 映射目录。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetectionConfidence {
    Weak,
    Strong,
    Exact,
    UserProvided,
}

pub struct CandidateRoot {
    pub path: PathBuf,
    pub kind: RootKind,
    pub confidence: DetectionConfidence,
    pub evidence: Vec<String>,
}
```

---

## 6. 数据模型

### 6.1 AppInstance

```rust
pub struct AppInstance {
    pub app: AppId,
    pub display_name: String,
    pub root: PathBuf,
    pub root_kind: RootKind,
    pub platform: OsKind,
    pub confidence: DetectionConfidence,
    pub evidence: Vec<String>,
}
```

### 6.2 InventoryItem

```rust
pub struct InventoryItem {
    pub id: String,
    pub app: AppId,
    pub path: PathBuf,
    pub kind: ItemKind,
    pub size_bytes: u64,
    pub modified_at: Option<OffsetDateTime>,
    pub risk: RiskLevel,
    pub recommendation: Recommendation,
    pub reason: String,
    pub backup_required: bool,
    pub parser: Option<String>,
}
```

### 6.3 AgentSession

```rust
pub struct AgentSession {
    pub id: String,
    pub app: AppId,
    pub title: Option<String>,
    pub cwd: Option<PathBuf>,
    pub started_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
    pub turns: Option<u32>,
    pub tokens: Option<TokenUsage>,
    pub files: Vec<PathBuf>,
    pub size_bytes: u64,
    pub preview: Option<String>,
    pub risk: RiskLevel,
    pub source: SessionSource,
}
```

### 6.4 风险等级

| 等级 | 含义 | 默认动作 |
|---|---|---|
| Green | 缓存、旧日志、临时文件、可重建索引 | 可进入 `--safe-only` 清理计划 |
| Yellow | 旧 session、归档 transcript、trace、checkpoint、workspace 历史 | 需要用户选择，执行前备份 |
| Red | 认证、配置、长期记忆、skills、commands、rules、核心 DB | 默认保留 |
| Black | 正在被占用、权限异常、未知二进制结构、校验失败 | 仅报告 |

---

## 7. Rust 架构

### 7.1 Workspace

```txt
vibe-cleaner/
├── crates/
│   ├── vclean-cli/             # clap commands, output, prompts
│   ├── vclean-core/            # models, risk engine, plan engine
│   ├── vclean-discovery/       # cross-platform path resolver
│   ├── vclean-adapters/        # app adapters
│   ├── vclean-parsers/         # jsonl/sqlite/leveldb/markdown/v8-subset
│   ├── vclean-cleaner/         # trash, backup, restore, vacuum planning
│   ├── vclean-redaction/       # secret masking
│   └── vclean-fixtures/        # test fixtures
├── src/main.rs
└── tests/
```

### 7.2 Adapter trait

```rust
pub trait AppAdapter: Send + Sync {
    fn id(&self) -> AppId;

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>>;

    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>>;

    fn sessions(&self, instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>>;

    fn clean_rules(&self) -> Vec<CleanRule>;
}
```

### 7.3 Parser trait

```rust
pub trait SessionParser: Send + Sync {
    fn name(&self) -> &'static str;

    fn supports(&self, input: &ParserInput) -> ParserSupport;

    fn parse(&self, input: ParserInput) -> anyhow::Result<Vec<AgentSession>>;
}
```

### 7.4 核心 crates

| 能力 | 推荐 crate | 用途 |
|---|---|---|
| CLI | `clap` | 子命令、参数、completion |
| 表格输出 | `tabled` 或 `comfy-table` | 终端报告 |
| JSON / JSONL | `serde`, `serde_json`, `simd-json` | session event 解析 |
| TOML | `toml` | Codex / vclean config |
| SQLite | `rusqlite` with `bundled`, `backup` | 只读 DB、schema introspection、备份 |
| LevelDB | `leveldb` / `leveldb-rs-binding` | Chromium localStorage / IndexedDB 底层读取 |
| 目录定位 | `directories`, `home`, `shellexpand` | 跨平台路径 |
| 文件遍历 | `walkdir`, `ignore`, `globset` | repo / home 扫描 |
| 时间 | `jiff` 或 `time` | 日期过滤和格式化 |
| 删除到回收站 | `trash` | OS Trash / Recycle Bin |
| 压缩备份 | `zstd`, `tar` | session / DB 备份 |
| 哈希 | `sha2`, `hex` | manifest 校验 |
| 进程检测 | `sysinfo` | app running guard |
| 错误处理 | `anyhow`, `thiserror` | 错误语义 |
| 日志 | `tracing` | debug / audit |
| 测试 | `insta`, `assert_cmd`, `tempfile` | snapshot / CLI / fixture |

---

## 8. 内置 Parser 设计

### 8.1 JSONL parser

目标：Claude Code、Codex、部分 OpenCode / Gemini session。

解析策略：

1. 逐行读取，单行失败进入 warning。
2. 抽取通用字段：`type`、`role`、`timestamp`、`cwd`、`session_id`、`message`、`summary`、`usage`。
3. 未知字段保留到 `raw_event_count` 和 `schema_fingerprint`。
4. title 取显式 summary，其次取第一条 user message。

### 8.2 Markdown parser

目标：Aider。

解析策略：

1. 根据 header、role marker、时间戳分段。
2. 从路径推断 repo root。
3. `--chat-history-file` 和 `--input-history-file` 可从配置或环境变量覆盖。
4. 只把 `.aider.chat.history.md` 视为 session 内容；`.aider.input.history` 视为输入历史。

### 8.3 SQLite introspector

目标：Goose、DeepChat、ALMA、Cursor state.vscdb、Cherry/Alma SQLite。

功能：

- 只读打开：`OpenFlags::SQLITE_OPEN_READ_ONLY`。
- 读取 `sqlite_master`、表体积、索引体积、列名、row count。
- 识别常见 KV 表：`ItemTable`、`cursorDiskKV`、`state`、`messages`、`conversations`、`sessions`。
- 对未知 schema 输出 “schema report”，生成 adapter 更新所需 fixture。

### 8.4 LevelDB localStorage parser

目标：Cherry Studio、Chromium/Electron localStorage。

策略：

1. 只读打开 LevelDB。
2. 搜索 key 中的 `persist:cherry-studio`、`redux`、`settings`、`providers` 等特征。
3. 对 value 做 Chromium localStorage 字符串解码：UTF-16LE / NUL-stripping / 控制字符清理。
4. 解析 redux-persist 的嵌套 JSON 字符串。
5. 提取 provider、assistant、topic metadata。

### 8.5 IndexedDB raw scanner + V8 subset deserializer

目标：Cherry Studio Dexie 的 `topics` 与 `message_blocks`。

首晚策略：

- 扫描 `.log`、`.ldb`、`.sst` 文件中的 V8 serialization header，例如 `0xFF 0x0F`。
- 实现 `v8_value_subset`，优先覆盖真实 fixture 中出现的类型：string、number、bool、null、array、plain object、object reference。
- 对每个 candidate slice 使用 bounded parsing：最大深度、最大 slice、最大对象数。
- 对 topic / block 做 shape detection：`id`、`messages`、`blocks`、`messageId`、`type`、`content`。
- 同一 ID 多版本保留最新候选：mtime 新、消息数多、content 长、block 数多。

这条路径保持 Rust 内置解析能力。Cherry Studio backup JSON parser 作为稳定入口，适合用户已有备份或 IndexedDB 子集 parser 覆盖不足的场景。

### 8.6 Cursor state.vscdb parser

目标：Cursor globalStorage / workspaceStorage。

策略：

1. 找到 `state.vscdb`。
2. 只读打开 SQLite。
3. 枚举 KV 表和 key family。
4. 识别 chat / composer / checkpoint / index / UI state 的 key pattern。
5. 输出 top keys by size 和 workspace 映射。
6. 首版只生成清理计划：backup、VACUUM、过期 workspaceStorage 归档。

### 8.7 Generic file sniffer

目标：Droid、OpenCode、Copilot、未知 agent。

策略：

- 根据文件名、magic bytes、JSON shape、mtime、目录结构生成候选 session。
- 常见字段：`sessionId`、`messages`、`turns`、`events`、`cwd`、`workspace`、`projectRoot`、`transcript`。
- 输出 `confidence`，低信心 session 默认只展示、不清理。

---

## 9. 各 App Adapter 细化

### 9.1 Claude Code

候选根：`CLAUDE_CONFIG_DIR` 或 `~/.claude`，Windows 为 `%USERPROFILE%\.claude`。

Inventory：

| 项 | 风险 | 动作 |
|---|---|---|
| `projects/**/*.jsonl` | Yellow | session browser；用户选择后备份+trash |
| `settings.json`, `settings.local.json` | Red | 保护 |
| `CLAUDE.md`, memory, skills, commands, agents | Red | 保护 |
| logs/cache/tmp | Green | 旧文件清理 |

Session parser：

- JSONL event scan。
- cwd 从 event 字段或 `projects/<encoded-cwd>` 反推。
- title 从 summary 或第一条 user prompt 提取。
- session id 取文件名 / event session id。

### 9.2 Codex

候选根：`CODEX_HOME` 或 `~/.codex`。

Inventory：

| 项 | 风险 | 动作 |
|---|---|---|
| `history.jsonl` | Yellow | 只读展示；按时间归档 |
| `sessions/**`, `archived_sessions/**` | Yellow | session browser；备份后选择清理 |
| `config.toml` | Red | 保护 |
| `auth.json` | Red | 保护 |
| logs/cache/tmp | Green | 清理 |

Parser：JSONL + TOML metadata + 目录日期 fallback。

### 9.3 OpenCode

候选根：

- Storage：`~/.local/share/opencode`，Windows `%USERPROFILE%\.local\share\opencode`。
- Config：`~/.config/opencode`，Windows `%USERPROFILE%\.config\opencode`。
- Cache：`~/.cache/opencode`。

Inventory：

| 项 | 风险 | 动作 |
|---|---|---|
| `log/` | Green | 旧日志清理 |
| cache / tmp | Green | 清理 |
| `auth.json` | Red | 保护 |
| `opencode.json(c)`、agents、commands、modes、plugins、skills、tools、themes | Red | 保护 |
| session storage | Yellow | 宽松 parser；选择清理 |

Parser：先做 JSON/JSONL/SQLite sniff，后续根据 fixture 固化 schema。

### 9.4 Cursor

候选根：

- macOS：`~/Library/Application Support/Cursor/User`
- Linux：`~/.config/Cursor/User`
- Windows：`%APPDATA%\Cursor\User`

重点文件：

| 文件 | 风险 | 策略 |
|---|---|---|
| `globalStorage/state.vscdb` | Red / Black | 只读扫描；展示 key family；生成 backup+VACUUM 计划 |
| `workspaceStorage/*/state.vscdb` | Yellow / Red | 映射 workspace；展示体积；过期 workspace 可归档 |
| cache / logs / GPUCache | Green | 可清理 |
| settings / keybindings / extensions state | Red | 保护 |

首版输出：

```txt
Cursor
  globalStorage/state.vscdb     1.24 GB   Red   contains composer/chat index
  workspaceStorage/3af...       812 MB    Yellow project=~/work/api updated=2026-04-21
  Cache                         433 MB    Green

Advanced plan available:
  1. Close Cursor
  2. Backup state.vscdb
  3. VACUUM INTO compact copy
  4. quick_check
  5. Replace atomically
```

### 9.5 Cherry Studio

来自既有真实数据分析的关键结构：

- macOS 数据目录：`~/Library/Application Support/CherryStudio/`
- `Local Storage/leveldb/`：redux-persist 状态，包含 providers、assistants。
- `IndexedDB/file__0.indexeddb.leveldb/`：Dexie 数据，包含 topics、message_blocks、settings。
- `Data/agents.db`：v2 新架构 SQLite，当前真实样本为空。
- `config.json`：客户端 ID。
- LocalStorage value 需要 UTF-16LE / NUL 清理。
- Cherry v7+ 的 topic metadata 与 message block 内容分离，message blocks 承载正文。

Adapter 策略：

| 路径 | 风险 | 动作 |
|---|---|---|
| `~/.cherrystudio/trace` 或 trace 目录 | Green | 可清理 |
| Cache / GPUCache / logs | Green | 可清理 |
| `Local Storage/leveldb` | Red | 只读解析，保护 |
| `IndexedDB/*.leveldb` | Red / Black | 只读解析，保护 |
| `Data/agents.db` | Red | SQLite introspection，保护 |
| backup JSON | Yellow | 用于 session browser，来源用户显式提供 |

Parser 优先级：

1. 用户提供 backup JSON：`vclean sessions list --app cherry --backup cherry-backup.json`。
2. LocalStorage LevelDB：提取 providers / assistants / topic metadata。
3. IndexedDB raw scanner：提取 topic / message_blocks 子集。
4. SQLite `agents.db` introspection：适配 v2 结构。

### 9.6 DeepChat

候选根：Electron userData。

核心结构：

| 文件 | 格式 | 策略 |
|---|---|---|
| `app_db/agent.db` | SQLite | 当前主库，只读解析 new_sessions/deepchat_messages，默认保护 |
| `app_db/chat.db` | SQLite | legacy conversations/messages fallback，默认保护 |
| app settings / Electron Store | JSON | Provider/config 保护 |
| DuckDB knowledge bases | DuckDB | 默认保护 |
| cache / logs | files | Green 清理 |

首版动作：只读 report + cache/log 清理。后续可加入 DeepChat session browser 和 backup/export。

### 9.7 Gemini CLI

候选根：`~/.gemini`。

重点路径：

| 路径 | 风险 | 动作 |
|---|---|---|
| `settings.json` | Red | 保护 |
| `tmp/<project_hash>/chats/` | Yellow | session 解析；过期 chat 可选择清理 |
| logs/cache | Green | 清理 |

Parser：JSON / JSONL / directory metadata。project hash 通过内容字段或用户提供 `--roots` 映射到项目目录。

### 9.8 Goose

候选路径：

- Unix-like：`~/.local/share/goose/sessions/sessions.db`
- Windows：`%APPDATA%\Block\goose\data\sessions\sessions.db`
- Config/history：`~/.config/goose/history.txt`

策略：

| 项 | 风险 | 动作 |
|---|---|---|
| `sessions.db` | Yellow / Red | SQLite 只读解析；清理需高级计划 |
| logs/cache | Green | 清理 |
| config/history | Red | 保护 |

### 9.9 Aider

Aider 是 repo-local 形态。默认历史文件：

- `.aider.chat.history.md`
- `.aider.input.history`
- `.aider.llm.history` 可选

策略：

| 项 | 风险 | 动作 |
|---|---|---|
| `.aider.chat.history.md` | Yellow | Markdown session browser；选择清理 |
| `.aider.input.history` | Yellow | 输入历史，选择清理 |
| `.aider.llm.history` | Yellow | 大体积 LLM raw history，选择清理 |
| `.aider.conf.yml` 等配置 | Red | 保护 |

### 9.10 ALMA

既有分析样本：

- macOS：`~/Library/Application Support/alma/`
- `chat_threads.db`：主 SQLite，包含 providers、chat_threads、chat_messages。
- `providers.db`：样本中为空，provider 数据位于 `chat_threads.db`。
- message 字段为 JSON，结构包含 `role` 与 `parts[].text`。

策略：

| 项 | 风险 | 动作 |
|---|---|---|
| `chat_threads.db` | Red | 只读解析 session；默认保护 |
| vector / memory / embedding store | Red | 保护 |
| logs/cache | Green | 清理 |

### 9.11 Factory Droid

候选根：

- User settings：`~/.factory/settings.json`
- User memory：`~/.factory/memories.md`
- Project settings：`.factory/settings.json`、`.factory/settings.local.json`

策略：

| 项 | 风险 | 动作 |
|---|---|---|
| settings / custom models / hooks | Red | 保护 |
| memories | Red | 保护 |
| project `.factory` | Red | 保护 |
| logs/tmp/session candidates | Yellow | 文件指纹探测；低信心只展示 |

Droid 首版保持 filesystem adapter。session raw layout 通过 fixture 增量固化，`listSessions()` 这类 SDK 能力用于研究 schema，不进入运行时。

---

## 10. CLI 设计

### 10.1 命令总览

```bash
vclean scan
vclean scan --apps claude,codex,opencode --json
vclean scan --roots cursor="D:\\CursorData" --include-windows-home

vclean sessions list --app claude --since 30d
vclean sessions list --cwd ~/work/my-repo --sort size
vclean sessions show <session-id> --format markdown
vclean sessions export <session-id> --format jsonl --output ./session.jsonl

vclean plan --safe-only
vclean plan --include-yellow --since 90d --apps claude,codex,aider
vclean clean --plan .vclean/plan-20260515.json --dry-run
vclean clean --plan .vclean/plan-20260515.json --execute --safe-only

vclean restore .vclean/manifests/20260515-claude-clean.json
vclean doctor
```

### 10.2 输出示意

```txt
$ vclean scan

Vibe Cleaner Scan
Local only · dry-run · built-in parsers · 12 adapters

┌──────────────┬─────────────┬──────────┬────────────┬──────────────┐
│ App          │ Root        │ Size     │ Candidates │ Recommendation │
├──────────────┼─────────────┼──────────┼────────────┼──────────────┤
│ Claude Code  │ ~/.claude   │ 3.42 GB  │ 184        │ 1.87 GB cleanable │
│ Codex        │ ~/.codex    │ 1.18 GB  │ 92         │ 622 MB cleanable  │
│ OpenCode     │ ~/.local/...│ 904 MB   │ 41         │ 388 MB cleanable  │
│ Cursor       │ App Support │ 11.6 GB  │ 9          │ backup + vacuum   │
│ CherryStudio │ userData    │ 2.24 GB  │ 17         │ trace/cache only  │
└──────────────┴─────────────┴──────────┴────────────┴──────────────┘

Green   2.9 GB  cache/log/tmp
Yellow  4.1 GB  old sessions/trace/checkpoints
Red     6.8 GB  credentials/config/memory/databases
Black   0.4 GB  locked or unknown
```

```txt
$ vclean sessions list --since 30d --sort size

┌────────┬──────────────┬────────────┬──────────────┬─────────┬──────────────┐
│ ID     │ App          │ Updated    │ Project      │ Size    │ Title        │
├────────┼──────────────┼────────────┼──────────────┼─────────┼──────────────┤
│ cc_83a │ Claude Code  │ 2026-05-13 │ ~/work/api   │ 281 MB  │ refactor auth │
│ cx_a19 │ Codex        │ 2026-05-11 │ ~/work/web   │ 133 MB  │ fix race cond │
│ gm_04d │ Gemini CLI   │ 2026-05-10 │ ~/work/cli   │ 89 MB   │ implement tui │
│ ad_9ce │ Aider        │ 2026-05-08 │ ~/oss/lib    │ 44 MB   │ migrate tests │
└────────┴──────────────┴────────────┴──────────────┴─────────┴──────────────┘
```

### 10.3 交互确认

```txt
$ vclean clean --plan .vclean/plan.json --execute

Plan summary
  Green   1.42 GB  93 files
  Yellow  0.00 GB  0 files
  Red     skipped

Execution mode
  Destination: OS Trash
  Backup:      ~/.local/share/vclean/backups/20260515-231722
  Manifest:    ~/.local/share/vclean/manifests/20260515-231722.json

Type CLEAN to continue: _
```

---

## 11. 清理策略

### 11.1 Green：默认可清理

- 旧 log 文件。
- cache / tmp / GPUCache / provider cache。
- 可重建 index。
- trace / diagnostic dump。
- 失败下载、临时备份、空目录。

### 11.2 Yellow：需要选择

- 旧 session transcript。
- archived session。
- workspaceStorage 中超过阈值且项目已不存在的目录。
- raw LLM history。
- checkpoint / restore points。
- SQLite compact 计划。

### 11.3 Red：默认保护

- credentials、auth、token、OAuth。
- config、settings、custom models。
- memory、rules、skills、commands、hooks。
- 核心 DB：聊天、知识库、agent state。
- 项目内 `.factory`、`AGENTS.md`、`CLAUDE.md`。

### 11.4 Black：只报告

- locked DB / LevelDB。
- unknown binary schema。
- permission denied。
- symlink escape 风险。
- hash 校验失败。

---

## 12. 安全执行模型

### 12.1 执行前检查

1. 进程检测：Cursor、Cherry Studio、DeepChat、Goose Desktop、OpenCode Desktop 等。
2. 文件锁检测：SQLite / LevelDB 只读打开，识别 busy / locked。
3. 路径安全：禁止 symlink escape，默认跟随 depth 受限。
4. 备份准备：Yellow / DB action 自动备份。
5. Manifest 生成：action、path、hash、size、mtime、backup path、trash id。

### 12.2 删除策略

```txt
try OS Trash
  ├─ success -> record trash action
  └─ failed
      ├─ copy to vclean quarantine -> record quarantine action
      └─ unsafe permanent delete requires --permanent --i-understand
```

### 12.3 Manifest

```json
{
  "id": "20260515-231722",
  "created_at": "2026-05-15T23:17:22+09:00",
  "platform": "macos",
  "actions": [
    {
      "app": "claude",
      "source": "/Users/me/.claude/projects/.../session.jsonl",
      "backup": "/Users/me/.local/share/vclean/backups/20260515/session.jsonl.zst",
      "sha256": "...",
      "size_bytes": 1048576,
      "action": "trash",
      "risk": "yellow"
    }
  ]
}
```

### 12.4 SQLite 高级清理

SQLite shrink 进入高级计划：

1. 检查目标 App 已退出。
2. 复制原 DB 到 backup。
3. 对 copy 执行 `PRAGMA integrity_check`。
4. 使用 `VACUUM INTO` 生成 compact DB。
5. 对 compact DB 执行 `PRAGMA quick_check`。
6. 原子替换。
7. manifest 记录原始 DB 和 compact DB hash。

首晚仅生成计划并展示潜在收益。

---

## 13. Session 解析与索引

### 13.1 标题提取

优先级：

1. 显式 title / name / summary 字段。
2. 第一条 user message 前 80 字。
3. 文件名或 session id。
4. cwd + 更新时间。

### 13.2 时间提取

优先级：

1. event timestamp。
2. metadata timestamp。
3. 文件修改时间。
4. 目录日期，例如 `sessions/YYYY/MM/DD`。

### 13.3 项目目录提取

优先级：

1. event 中的 cwd / workspace / root 字段。
2. Claude Code encoded-cwd 路径反解。
3. workspace metadata，例如 Cursor workspace storage。
4. 用户通过 `--roots` 提供映射。

### 13.4 索引字段

| 字段 | 用途 |
|---|---|
| `app` | 按工具筛选 |
| `cwd` | 按项目筛选 |
| `title` | 搜索 / 展示 |
| `updated_at` | 按时间清理 |
| `size_bytes` | 找出最大 session |
| `turns` | 衡量历史长度 |
| `tokens` | 成本分析 |
| `risk` | 控制默认动作 |
| `files` | 归档 / 备份 |
| `preview_hash` | 隐私友好去重 |

### 13.5 隐私与 redaction

默认 preview 只展示前 200 字，且自动遮盖：

- API key：OpenAI、Anthropic、GitHub、Google、AWS 等。
- Bearer token。
- URL credentials。
- SSH private key。
- JWT。
- 电子邮件、手机号可选遮盖。

`--raw` 需要二次确认。

---

## 14. Cherry / ALMA 既有分析融入方案

### 14.1 ALMA parser

数据结构：`chat_threads.db` 包含 `providers`、`chat_threads`、`chat_messages`。`chat_messages.message` 是 JSON，包含 `role` 与 `parts[].text`。

Rust parser：

```rust
pub struct AlmaSqliteParser;

impl AlmaSqliteParser {
    pub fn parse_threads(conn: &rusqlite::Connection) -> anyhow::Result<Vec<AgentSession>> {
        // Query chat_threads and join chat_messages by thread_id.
        // Extract role and text from JSON message.parts.
        todo!()
    }
}
```

清理策略：

- `chat_threads.db`：Red，只读解析。
- logs/cache：Green。
- provider / API key：Red，遮盖展示。

### 14.2 Cherry Studio parser

数据结构：

- Provider / Assistant：LocalStorage LevelDB 中的 redux-persist。
- Topic / MessageBlock：IndexedDB Dexie LevelDB。
- Message -> Block：`Message.blocks` 存 block id，正文在 `message_blocks`。
- Backup JSON：`localStorage` + `indexedDB.topics` + `indexedDB.message_blocks`。

Rust parser 拆成 4 个模块：

```txt
cherry/
  ├─ backup_json.rs          # stable JSON backup parser
  ├─ local_storage.rs        # LevelDB + string decoder + redux-persist parser
  ├─ indexeddb_scan.rs       # .log/.ldb scanner
  └─ v8_value_subset.rs      # subset deserializer for observed shapes
```

首晚落地重点：

1. Backup JSON parser：完整解析 topics / message_blocks。
2. LocalStorage parser：提取 provider / assistant metadata。
3. IndexedDB scanner：统计候选对象数量，能还原真实 fixture 中的 topic/block。
4. 清理动作限定 trace/cache。

---

## 15. 当晚实施切片

### 切片 A：项目骨架

- Rust workspace。
- `clap` 子命令。
- `AppAdapter`、`SessionParser`、`PathResolver`。
- `RiskLevel`、`InventoryItem`、`AgentSession`。

验收：

```bash
cargo run -p vclean-cli -- scan --help
cargo run -p vclean-cli -- sessions list --help
```

### 切片 B：跨平台路径探测

- `PathContext`。
- macOS / Linux / Windows / WSL candidate roots。
- `--portable-root` fixture 模式。
- `--roots app=path` 覆盖。

验收：在三类 fixture home 下输出相同结构的 scan report。

### 切片 C：核心 parser

- JSONL parser：Claude / Codex / Gemini。
- Markdown parser：Aider。
- SQLite introspector：Goose / DeepChat / Cursor / ALMA。
- LocalStorage decoder：Cherry。

验收：`vclean sessions list --since 30d --sort size` 显示 title、cwd、time、size。

### 切片 D：risk engine + plan

- Green / Yellow / Red / Black 分类。
- 规则文件内置。
- plan JSON 输出。
- sensitive preview redaction。

验收：plan 中包含 reason 和 backup_required。

### 切片 E：backup + trash + restore

- `trash` crate 集成。
- zstd backup。
- sha256 校验。
- manifest。
- restore。

验收：fixture 清理后可完整恢复。

### 切片 F：保护性扫描

- Cursor state.vscdb 只读报告。
- Cherry userData / trace / LocalStorage 只读报告。
- DeepChat app_db / cache 只读报告。
- ALMA SQLite 只读报告。
- Droid `.factory` 保护性报告。

验收：输出“主要占用来源 + 建议动作 + 保护原因”。

---

## 16. 测试方案

### 16.1 Fixture 驱动

```txt
fixtures/
├── macos-home/
│   ├── .claude/
│   ├── .codex/
│   ├── .gemini/
│   └── Library/Application Support/CherryStudio/
├── linux-home/
│   ├── .local/share/opencode/
│   ├── .config/Cursor/User/
│   └── .local/share/goose/
└── windows-home/
    ├── AppData/Roaming/Cursor/User/
    ├── AppData/Roaming/DeepChat/
    └── .codex/
```

### 16.2 测试类别

| 测试 | 内容 |
|---|---|
| Unit | parser、path resolver、risk rules、redaction |
| Snapshot | `scan`、`sessions list` 输出 |
| Mutation | dry-run 前后 hash 保持一致 |
| SQLite | readonly、locked DB、schema fallback |
| LevelDB | localStorage 解码、locked DB 行为 |
| Restore | trash/quarantine/backup 回滚 |
| Cross-platform | Windows path、UNC path、WSL path、symlink |

### 16.3 回归样本

- Claude JSONL 多行未知事件。
- Codex `history.jsonl` 和 session 目录。
- Gemini tmp chats。
- Aider markdown history。
- Cherry backup v5。
- Cherry LocalStorage UTF-16LE / NUL 样本。
- Cursor `state.vscdb` KV 样本。
- ALMA `chat_threads.db` 样本。

---

## 17. 配置文件

位置：

| OS | 配置路径 |
|---|---|
| macOS | `~/Library/Application Support/vclean/config.toml` 或 `~/.config/vclean/config.toml` |
| Linux | `${XDG_CONFIG_HOME:-~/.config}/vclean/config.toml` |
| Windows | `%APPDATA%\vclean\config.toml` |

示例：

```toml
[general]
default_days = 30
use_trash = true
backup_before_delete = true
local_only = true
show_raw_preview = false

[scan]
follow_symlinks = false
max_depth = 8
include_windows_home_from_wsl = false

[apps.claude]
enabled = true
custom_roots = []

[apps.codex]
enabled = true
codex_home = ""

[apps.cursor]
enabled = true
advanced_sqlite_cleanup = false

[apps.cherry]
enabled = true
backup_files = []
indexeddb_subset_parser = true

[redaction]
mask_api_keys = true
mask_bearer_tokens = true
mask_url_credentials = true
```

---

## 18. 发布路线

### v0.1：安全扫描版

- Rust CLI。
- 跨平台路径 resolver。
- Claude / Codex / Gemini / Aider session browser。
- OpenCode log/cache/session sniff。
- Cursor / Cherry / DeepChat / ALMA / Droid 保护性扫描。
- plan / backup / trash / restore。

### v0.2：结构化解析增强

- Goose sessions.db 完整 parser。
- Cursor key family parser + workspace 映射。
- Cherry IndexedDB V8 subset parser 增强。
- DeepChat session browser。
- Droid session raw layout 固化。

### v0.3：TUI

- `ratatui` 多选清理。
- session preview。
- 按项目 / 时间 / 体积 / 工具筛选。
- 风险解释面板。

```txt
┌ Vibe Cleaner ───────────────────────────────────────────────┐
│ Filters: [App: All] [Since: 90d] [Risk: Green+Yellow]       │
├──────────────┬────────────┬──────────────┬─────────┬───────┤
│ App          │ Updated    │ Project      │ Size    │ Pick  │
├──────────────┼────────────┼──────────────┼─────────┼───────┤
│ Claude Code  │ 2026-05-13 │ ~/work/api   │ 281 MB  │  [x]  │
│ Codex        │ 2026-05-11 │ ~/work/web   │ 133 MB  │  [x]  │
│ Cursor       │ 2026-04-29 │ ~/work/app   │ 1.2 GB  │  [ ]  │
├──────────────┴────────────┴──────────────┴─────────┴───────┤
│ Preview: refactor auth flow...                             │
│ Reason: old session transcript, backup required             │
└──────────────────────────────────────────────────────────────┘
```

### v1.0：Agent data control center

- Adapter registry。
- Session export：Markdown / JSONL / HTML。
- Privacy report：本地 agent 数据中的 secrets 风险。
- Cross-machine archive / restore。
- Repo-level storage accounting。

---

## 19. 首晚推荐落地顺序

执行顺序：

1. `PathResolver` + fixture home。
2. `scan` inventory + 风险分类。
3. Claude / Codex / Gemini JSONL parser。
4. Aider Markdown parser。
5. plan + backup + trash + manifest + restore。
6. Cursor / Cherry / DeepChat / ALMA / Droid 保护性扫描。
7. Cherry backup JSON parser + LocalStorage decoder。
8. 把 IndexedDB V8 subset parser 放进 v0.2 分支，但首晚可完成 scanner 与 fixture 测试壳。

最终首晚交付物：一个安全可用的 `vclean`，能在真实机器上给出可解释的体积和清理建议，并对主流 CLI agent 的 session 做多维浏览和安全清理。🦀

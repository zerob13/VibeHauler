#![allow(
    clippy::cast_precision_loss,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]

use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    widgets::{Clear, Paragraph},
};
use vibe_hauler_adapters::registry::AdapterRegistry;
use vibe_hauler_cleaner::{CleanerConfig, ExecutionMode, FilesystemCleaner, PlanExecutor};
use vibe_hauler_config::{VibeHaulerConfig, default_data_dir};
use vibe_hauler_core::{
    AgentSession, AppId, AppInstance, CleanManifest, CleanPlan, InventoryItem, OsKind, PlanMode,
    PlanPolicy, RiskLevel, RootKind, now_rfc3339,
};
use vibe_hauler_discovery::PathContext;

#[derive(Debug, Parser)]
#[command(name = "vhaul", version, about = "Launch the VibeHauler TUI.")]
pub struct Cli {
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long)]
    pub no_color: bool,

    #[arg(long)]
    pub portable_root: Option<PathBuf>,

    #[arg(long)]
    pub roots: Option<String>,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut app = TuiApp::new(cli)?;
    if !io::stdout().is_terminal() {
        println!("{}", app.render());
        return Ok(());
    }
    run_terminal(&mut app)
}

fn run_terminal(app: &mut TuiApp) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let result = loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Clear, area);
            frame.render_widget(Paragraph::new(app.render()), area);
        })?;
        if app.should_quit {
            break Ok(());
        }
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key)?;
        }
    };
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    AppSelection,
    Analyze,
    SafeSelection,
    SafeExecution,
    SessionOverview,
    SessionBrowser,
    SessionConfirmation,
    SessionExecution,
    FinalSummary,
}

struct AppRow {
    instance: AppInstance,
    selected: bool,
    selectable: bool,
}

struct TuiApp {
    cli: Cli,
    registry: AdapterRegistry,
    screen: Screen,
    app_rows: Vec<AppRow>,
    app_cursor: usize,
    inventory: Vec<InventoryItem>,
    sessions: Vec<AgentSession>,
    safe_selected: Vec<bool>,
    safe_cursor: usize,
    session_cursor: usize,
    browser_app: Option<AppId>,
    session_selected: BTreeSet<usize>,
    confirmation_input: String,
    safe_manifest: Option<CleanManifest>,
    session_manifest: Option<CleanManifest>,
    error: Option<String>,
    should_quit: bool,
}

impl TuiApp {
    fn new(cli: Cli) -> anyhow::Result<Self> {
        let config = VibeHaulerConfig::load(cli.config.as_deref())?;
        let os = cli
            .portable_root
            .as_deref()
            .map_or_else(OsKind::current, PathContext::infer_portable_os);
        let ctx = if let Some(root) = &cli.portable_root {
            PathContext::for_portable_root(root, os)
        } else {
            PathContext::from_env()?
        };
        let registry = AdapterRegistry::v01();
        let mut instances = registry.detect_all(&ctx)?;
        instances.extend(user_root_instances(cli.roots.as_ref(), os)?);
        instances.extend(config_root_instances(&config, os));
        instances.sort_by(|left, right| {
            left.app
                .cmp(&right.app)
                .then_with(|| left.root.cmp(&right.root))
        });
        instances.dedup_by(|left, right| left.app == right.app && left.root == right.root);

        let app_rows = instances
            .into_iter()
            .filter(|instance| config.app_enabled(&instance.app))
            .map(|instance| AppRow {
                selected: true,
                selectable: true,
                instance,
            })
            .collect::<Vec<_>>();

        Ok(Self {
            cli,
            registry,
            screen: Screen::AppSelection,
            app_rows,
            app_cursor: 0,
            inventory: Vec::new(),
            sessions: Vec::new(),
            safe_selected: Vec::new(),
            safe_cursor: 0,
            session_cursor: 0,
            browser_app: None,
            session_selected: BTreeSet::new(),
            confirmation_input: String::new(),
            safe_manifest: None,
            session_manifest: None,
            error: None,
            should_quit: false,
        })
    }

    fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Char('?') => {
                self.error =
                    Some("Keys: Up/Down move, Space toggle, Enter continue, q quit".to_owned());
                return Ok(());
            }
            _ => {}
        }

        match self.screen {
            Screen::AppSelection => self.handle_app_selection(key),
            Screen::Analyze | Screen::SafeExecution | Screen::SessionExecution => Ok(()),
            Screen::SafeSelection => self.handle_safe_selection(key),
            Screen::SessionOverview => self.handle_session_overview(key),
            Screen::SessionBrowser => self.handle_session_browser(key),
            Screen::SessionConfirmation => self.handle_session_confirmation(key),
            Screen::FinalSummary => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
                    self.should_quit = true;
                }
                Ok(())
            }
        }
    }

    fn handle_app_selection(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Down => self.app_cursor = next_cursor(self.app_cursor, self.app_rows.len()),
            KeyCode::Up => self.app_cursor = prev_cursor(self.app_cursor, self.app_rows.len()),
            KeyCode::Char('a') => self
                .app_rows
                .iter_mut()
                .filter(|row| row.selectable)
                .for_each(|row| row.selected = true),
            KeyCode::Char('n') => self
                .app_rows
                .iter_mut()
                .filter(|row| row.selectable)
                .for_each(|row| row.selected = false),
            KeyCode::Char(' ') => {
                if let Some(row) = self.app_rows.get_mut(self.app_cursor)
                    && row.selectable
                {
                    row.selected = !row.selected;
                }
            }
            KeyCode::Enter => {
                if self.app_rows.iter().any(|row| row.selected) {
                    self.screen = Screen::Analyze;
                    self.analyze_selected()?;
                    self.screen = Screen::SafeSelection;
                } else {
                    self.error =
                        Some("No apps selected. Select at least one app to continue.".to_owned());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_safe_selection(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Down => {
                self.safe_cursor = next_cursor(self.safe_cursor, self.safe_items().len());
            }
            KeyCode::Up => {
                self.safe_cursor = prev_cursor(self.safe_cursor, self.safe_items().len());
            }
            KeyCode::Char('a') => self
                .safe_selected
                .iter_mut()
                .for_each(|selected| *selected = true),
            KeyCode::Char('n') => self
                .safe_selected
                .iter_mut()
                .for_each(|selected| *selected = false),
            KeyCode::Char(' ') => {
                if let Some(selected) = self.safe_selected.get_mut(self.safe_cursor) {
                    *selected = !*selected;
                }
            }
            KeyCode::Enter => {
                self.screen = Screen::SafeExecution;
                self.execute_safe_cleanup()?;
                self.screen = Screen::SessionOverview;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_overview(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Char('s') => self.screen = Screen::FinalSummary,
            KeyCode::Down => {
                self.session_cursor = next_cursor(self.session_cursor, self.session_apps().len());
            }
            KeyCode::Up => {
                self.session_cursor = prev_cursor(self.session_cursor, self.session_apps().len());
            }
            KeyCode::Enter => {
                if let Some(app) = self.session_apps().get(self.session_cursor).cloned() {
                    self.browser_app = Some(app);
                    self.screen = Screen::SessionBrowser;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_browser(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        let visible = self.visible_session_indices();
        match key.code {
            KeyCode::Esc => self.screen = Screen::SessionOverview,
            KeyCode::Down => self.session_cursor = next_cursor(self.session_cursor, visible.len()),
            KeyCode::Up => self.session_cursor = prev_cursor(self.session_cursor, visible.len()),
            KeyCode::Char('a') => visible.iter().for_each(|index| {
                self.session_selected.insert(*index);
            }),
            KeyCode::Char('n') => visible.iter().for_each(|index| {
                self.session_selected.remove(index);
            }),
            KeyCode::Char(' ') => {
                if let Some(index) = visible.get(self.session_cursor)
                    && !self.session_selected.remove(index)
                {
                    self.session_selected.insert(*index);
                }
            }
            KeyCode::Enter => {
                if self.selected_session_count() > 0 {
                    self.confirmation_input.clear();
                    self.screen = Screen::SessionConfirmation;
                } else {
                    self.error = Some("Select at least one session before cleanup.".to_owned());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_confirmation(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => self.screen = Screen::SessionBrowser,
            KeyCode::Backspace => {
                self.confirmation_input.pop();
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                self.confirmation_input.push(ch);
            }
            KeyCode::Enter => {
                if self.confirmation_input == self.selected_session_count().to_string() {
                    self.screen = Screen::SessionExecution;
                    self.execute_session_cleanup()?;
                    self.screen = Screen::FinalSummary;
                } else {
                    self.error =
                        Some("Typed confirmation did not match selected count.".to_owned());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze_selected(&mut self) -> anyhow::Result<()> {
        self.inventory.clear();
        self.sessions.clear();
        for row in self.app_rows.iter().filter(|row| row.selected) {
            let Some(adapter) = self.registry.adapter_for(&row.instance.app) else {
                continue;
            };
            self.inventory.extend(adapter.inventory(&row.instance)?);
            self.sessions.extend(adapter.sessions(&row.instance)?);
        }
        self.safe_selected = self.safe_items().iter().map(|_| true).collect();
        Ok(())
    }

    fn execute_safe_cleanup(&mut self) -> anyhow::Result<()> {
        let selected_items = self
            .safe_items()
            .into_iter()
            .enumerate()
            .filter_map(|(index, item)| {
                self.safe_selected
                    .get(index)
                    .copied()
                    .unwrap_or(false)
                    .then_some(item.clone())
            })
            .collect::<Vec<_>>();
        if selected_items.is_empty() {
            return Ok(());
        }
        let plan = CleanPlan::build(
            &selected_items,
            self.platform(),
            PlanMode::SafeCleanup,
            &PlanPolicy::safe_cleanup(),
        )?;
        let cleaner = FilesystemCleaner::new(self.cleaner_config()?);
        self.safe_manifest = Some(cleaner.execute(&plan, ExecutionMode::Execute)?);
        Ok(())
    }

    fn execute_session_cleanup(&mut self) -> anyhow::Result<()> {
        let selected_sessions = self
            .session_selected
            .iter()
            .filter_map(|index| self.sessions.get(*index).cloned())
            .collect::<Vec<_>>();
        if selected_sessions.is_empty() {
            return Ok(());
        }
        let plan =
            CleanPlan::build_for_sessions(&selected_sessions, self.platform(), &now_rfc3339())?;
        let cleaner = FilesystemCleaner::new(self.cleaner_config()?);
        self.session_manifest = Some(cleaner.execute(&plan, ExecutionMode::Execute)?);
        Ok(())
    }

    fn cleaner_config(&self) -> anyhow::Result<CleanerConfig> {
        let data_dir = default_data_dir(self.platform(), self.cli.portable_root.as_deref())
            .context("could not determine VibeHauler data directory")?;
        let mut app_roots: BTreeMap<AppId, Vec<PathBuf>> = BTreeMap::new();
        for row in self.app_rows.iter().filter(|row| row.selected) {
            app_roots
                .entry(row.instance.app.clone())
                .or_default()
                .push(row.instance.root.clone());
        }
        Ok(CleanerConfig {
            data_dir,
            app_roots,
            use_trash: true,
            backup_space_override: None,
        })
    }

    fn platform(&self) -> OsKind {
        self.cli
            .portable_root
            .as_deref()
            .map_or_else(OsKind::current, PathContext::infer_portable_os)
    }

    fn safe_items(&self) -> Vec<&InventoryItem> {
        self.inventory
            .iter()
            .filter(|item| item.risk == RiskLevel::Green)
            .collect()
    }

    fn protected_items(&self) -> usize {
        self.inventory
            .iter()
            .filter(|item| matches!(item.risk, RiskLevel::Red | RiskLevel::Black))
            .count()
    }

    fn session_apps(&self) -> Vec<AppId> {
        let mut apps = self
            .sessions
            .iter()
            .map(|session| session.app.clone())
            .collect::<Vec<_>>();
        apps.sort();
        apps.dedup();
        apps
    }

    fn visible_session_indices(&self) -> Vec<usize> {
        let Some(app) = &self.browser_app else {
            return Vec::new();
        };
        self.sessions
            .iter()
            .enumerate()
            .filter_map(|(index, session)| (&session.app == app).then_some(index))
            .collect()
    }

    fn selected_session_count(&self) -> usize {
        self.session_selected.len()
    }

    fn render(&self) -> String {
        let mut output = match self.screen {
            Screen::AppSelection => self.render_app_selection(),
            Screen::Analyze => self.render_analysis(),
            Screen::SafeSelection => self.render_safe_selection(),
            Screen::SafeExecution => self.render_safe_execution(),
            Screen::SessionOverview => self.render_session_overview(),
            Screen::SessionBrowser => self.render_session_browser(),
            Screen::SessionConfirmation => self.render_session_confirmation(),
            Screen::SessionExecution => self.render_session_execution(),
            Screen::FinalSummary => self.render_final_summary(),
        };
        if let Some(error) = &self.error {
            output.push_str("\n\n");
            output.push_str(error);
        }
        output
    }

    fn render_app_selection(&self) -> String {
        let mut lines = vec![
            "+-- VibeHauler ----------------------------- Local-only cleanup --+".to_owned(),
            "| Select apps to inspect                                          |".to_owned(),
            "| All detected apps are selected by default.                      |".to_owned(),
            "+----+------------------+--------------------------+--------------+".to_owned(),
            "|    | App              | Data root                | Status       |".to_owned(),
            "+----+------------------+--------------------------+--------------+".to_owned(),
        ];
        if self.app_rows.is_empty() {
            lines.push(
                "|    | No supported apps found in this home                       |".to_owned(),
            );
        } else {
            for (index, row) in self.app_rows.iter().enumerate() {
                lines.push(format!(
                    "| {}  | [{}] {:12} | {:24} | {:12} |",
                    if index == self.app_cursor { ">" } else { " " },
                    if row.selected { "x" } else { " " },
                    truncate(row.instance.display_name.as_str(), 12),
                    truncate(display_path(&row.instance.root), 24),
                    "found"
                ));
            }
        }
        lines.extend([
            "+----+------------------+--------------------------+--------------+".to_owned(),
            format!(
                "| Selected: {:<53} |",
                format!(
                    "{} apps",
                    self.app_rows.iter().filter(|row| row.selected).count()
                )
            ),
            "| Enter Continue - Space Toggle - ? Help - q Quit                 |".to_owned(),
            "+-----------------------------------------------------------------+".to_owned(),
        ]);
        lines.join("\n")
    }

    fn render_analysis(&self) -> String {
        "+-- Analyzing selected apps --------------------------------------+\n| Reading inventory and sessions...                               |\n+-----------------------------------------------------------------+".to_owned()
    }

    fn render_safe_selection(&self) -> String {
        let safe = self.safe_items();
        let selected_bytes = safe
            .iter()
            .enumerate()
            .filter(|(index, _)| self.safe_selected.get(*index).copied().unwrap_or(false))
            .map(|(_, item)| item.size_bytes)
            .sum::<u64>();
        let mut lines = vec![
            "+-- Safe Cleanup -------------------------------------------------+".to_owned(),
            "| Rebuildable data selected by default                            |".to_owned(),
            format!(
                "| Selected: {:<54} |",
                format!(
                    "{} - {} files",
                    human_bytes(selected_bytes),
                    selected_count(&self.safe_selected)
                )
            ),
            "+----+--------------------------+-------------+--------+----------+".to_owned(),
            "|    | Item                     | App         | Size   | Why      |".to_owned(),
            "+----+--------------------------+-------------+--------+----------+".to_owned(),
        ];
        if safe.is_empty() {
            lines.push(
                "|    | No rebuildable cleanup candidates found                    |".to_owned(),
            );
        } else {
            for (index, item) in safe.iter().enumerate() {
                lines.push(format!(
                    "| {}  | [{}] {:20} | {:11} | {:6} | {:8} |",
                    if index == self.safe_cursor { ">" } else { " " },
                    if self.safe_selected.get(index).copied().unwrap_or(false) {
                        "x"
                    } else {
                        " "
                    },
                    truncate(file_label(&item.path), 20),
                    truncate(item.app.display_name(), 11),
                    truncate(human_bytes(item.size_bytes), 6),
                    truncate(&item.reason, 8),
                ));
            }
        }
        lines.extend([
            "+----+--------------------------+-------------+--------+----------+".to_owned(),
            format!(
                "| Protected: {:<53} |",
                format!("{} Red/Black items are report-only", self.protected_items())
            ),
            "| Enter Clean selected - Space Toggle - q Quit                    |".to_owned(),
            "+-----------------------------------------------------------------+".to_owned(),
        ]);
        lines.join("\n")
    }

    fn render_safe_execution(&self) -> String {
        "+-- Cleaning safe items ------------------------------------------+\n| Moving selected Green items to managed Trash...                  |\n+-----------------------------------------------------------------+".to_owned()
    }

    fn render_session_overview(&self) -> String {
        let apps = self.session_apps();
        let mut lines = vec![
            "+-- Session Data -------------------------------------------------+".to_owned(),
            "| Agent sessions are history/transcripts/checkpoints.             |".to_owned(),
            "+----+------------------+----------+--------+---------------------+".to_owned(),
            "|    | App              | Sessions | Size   | Newest              |".to_owned(),
            "+----+------------------+----------+--------+---------------------+".to_owned(),
        ];
        if apps.is_empty() {
            lines.push(
                "|    | No reviewable session data found                           |".to_owned(),
            );
        } else {
            for (index, app) in apps.iter().enumerate() {
                let app_sessions = self
                    .sessions
                    .iter()
                    .filter(|session| &session.app == app)
                    .collect::<Vec<_>>();
                let size = app_sessions
                    .iter()
                    .map(|session| session.size_bytes)
                    .sum::<u64>();
                let newest = app_sessions
                    .iter()
                    .filter_map(|session| session.updated_at.as_deref())
                    .max()
                    .unwrap_or("unknown");
                lines.push(format!(
                    "| {}  | {:16} | {:8} | {:6} | {:19} |",
                    if index == self.session_cursor {
                        ">"
                    } else {
                        " "
                    },
                    truncate(app.display_name(), 16),
                    app_sessions.len(),
                    truncate(human_bytes(size), 6),
                    truncate(newest, 19),
                ));
            }
        }
        lines.extend([
            "+----+------------------+----------+--------+---------------------+".to_owned(),
            "| Enter Open app - s Skip session cleanup - q Quit                |".to_owned(),
            "+-----------------------------------------------------------------+".to_owned(),
        ]);
        lines.join("\n")
    }

    fn render_session_browser(&self) -> String {
        let visible = self.visible_session_indices();
        let title = self
            .browser_app
            .as_ref()
            .map_or("Sessions".to_owned(), |app| {
                format!("Sessions / {}", app.display_name())
            });
        let mut lines = vec![
            format!("+-- {:<58}+", truncate(&title, 58)),
            format!(
                "| {:<63} |",
                format!(
                    "{} sessions - {} selected",
                    visible.len(),
                    visible
                        .iter()
                        .filter(|index| self.session_selected.contains(index))
                        .count()
                )
            ),
            "+----+------------------+------------------------+-------+--------+".to_owned(),
            "|    | Updated          | Directory              | Size  | Title  |".to_owned(),
            "+----+------------------+------------------------+-------+--------+".to_owned(),
        ];
        for (row, session_index) in visible.iter().enumerate() {
            let session = &self.sessions[*session_index];
            lines.push(format!(
                "| {}  | [{}] {:12} | {:22} | {:5} | {:6} |",
                if row == self.session_cursor { ">" } else { " " },
                if self.session_selected.contains(session_index) {
                    "x"
                } else {
                    " "
                },
                truncate(session.updated_at.as_deref().unwrap_or("unknown"), 12),
                truncate(
                    session
                        .cwd
                        .as_ref()
                        .map_or("unknown".to_owned(), |path| display_path(path)),
                    22
                ),
                truncate(human_bytes(session.size_bytes), 5),
                truncate(session.title.as_deref().unwrap_or("untitled"), 6),
            ));
        }
        let preview = visible
            .get(self.session_cursor)
            .and_then(|index| self.sessions.get(*index))
            .and_then(|session| session.preview.as_deref())
            .unwrap_or("");
        lines.extend([
            "+----+------------------+------------------------+-------+--------+".to_owned(),
            format!("| Preview: {:<54} |", truncate(preview, 54)),
            "| Space Toggle - Enter Review selected - Esc Back                 |".to_owned(),
            "+-----------------------------------------------------------------+".to_owned(),
        ]);
        lines.join("\n")
    }

    fn render_session_confirmation(&self) -> String {
        format!(
            "+-- Confirm session cleanup --------------------------------------+\n| Selected Yellow sessions: {:<38} |\n| Type the selected count to confirm: {:<29} |\n| Backup is required before Trash. Esc cancels.                    |\n+-----------------------------------------------------------------+",
            self.selected_session_count(),
            self.confirmation_input
        )
    }

    fn render_session_execution(&self) -> String {
        "+-- Cleaning session data ----------------------------------------+\n| Backing up selected Yellow sessions, then moving to Trash...     |\n+-----------------------------------------------------------------+".to_owned()
    }

    fn render_final_summary(&self) -> String {
        let safe = self
            .safe_manifest
            .as_ref()
            .map_or("none".to_owned(), |manifest| manifest.id.clone());
        let sessions = self
            .session_manifest
            .as_ref()
            .map_or("skipped".to_owned(), |manifest| manifest.id.clone());
        format!(
            "+-- Final Summary ------------------------------------------------+\n| Safe cleanup manifest: {:<39} |\n| Session cleanup manifest: {:<36} |\n| Restore: use the manifest backups under .vibe-hauler/backups.    |\n| Press Enter or q to exit.                                       |\n+-----------------------------------------------------------------+",
            truncate(&safe, 39),
            truncate(&sessions, 36)
        )
    }
}

fn next_cursor(current: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (current + 1).min(len - 1)
    }
}

fn prev_cursor(current: usize, _len: usize) -> usize {
    current.saturating_sub(1)
}

fn selected_count(values: &[bool]) -> usize {
    values.iter().filter(|value| **value).count()
}

fn truncate(value: impl AsRef<str>, max: usize) -> String {
    let value = value.as_ref();
    if value.chars().count() <= max {
        format!("{value:<max$}")
    } else if max <= 3 {
        value.chars().take(max).collect()
    } else {
        let keep = max - 3;
        format!("{}...", value.chars().take(keep).collect::<String>())
    }
}

fn human_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format!("{:.1}GB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1}MB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1}KB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes}B")
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| display_path(path), str::to_owned)
}

fn user_root_instances(roots: Option<&String>, os: OsKind) -> anyhow::Result<Vec<AppInstance>> {
    let Some(roots) = roots else {
        return Ok(Vec::new());
    };
    roots
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let (app, path) = part
                .split_once('=')
                .context("--roots entries must use app=path")?;
            Ok(instance_from_root(
                AppId::from_key(app),
                PathBuf::from(path),
                RootKind::UserProvided,
                os,
                "user provided root",
            ))
        })
        .collect()
}

fn config_root_instances(config: &VibeHaulerConfig, os: OsKind) -> Vec<AppInstance> {
    let mut instances = Vec::new();
    for key in config.apps.keys() {
        let app = AppId::from_key(key);
        for root in config.app_custom_roots(&app) {
            instances.push(instance_from_root(
                app.clone(),
                root,
                RootKind::UserProvided,
                os,
                "config custom root",
            ));
        }
    }
    instances
}

fn instance_from_root(
    app: AppId,
    root: PathBuf,
    root_kind: RootKind,
    os: OsKind,
    evidence: &str,
) -> AppInstance {
    let root = root.canonicalize().unwrap_or(root);
    AppInstance {
        id: format!("inst_{}", app.key()),
        display_name: app.display_name().to_owned(),
        app,
        root,
        root_kind,
        platform: os,
        confidence: vibe_hauler_core::DetectionConfidence::UserProvided,
        evidence: vec![evidence.to_owned()],
    }
}

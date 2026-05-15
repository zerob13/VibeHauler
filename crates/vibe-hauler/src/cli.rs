#![allow(
    clippy::cast_precision_loss,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
    fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};
use vibe_hauler_adapters::registry::AdapterRegistry;
use vibe_hauler_cleaner::{CleanerConfig, ExecutionMode, FilesystemCleaner, PlanExecutor};
use vibe_hauler_config::{VibeHaulerConfig, default_data_dir};
use vibe_hauler_core::{
    AgentSession, AppId, AppInstance, CleanManifest, CleanPlan, InventoryItem, OsKind, PlanMode,
    PlanPolicy, RiskLevel, RootKind, SessionSource, now_rfc3339,
};
use vibe_hauler_discovery::PathContext;

const SESSION_BROWSER_PAGE_SIZE: usize = 11;

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
        app.discover()?;
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
    let result = run_terminal_loop(&mut terminal, app);
    let cleanup = cleanup_terminal(&mut terminal);
    result.and(cleanup)
}

fn run_terminal_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| {
            render_tui(frame, app);
        })?;
        if app.screen == Screen::Boot {
            app.discover()?;
            continue;
        }
        if app.should_quit {
            break Ok(());
        }
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key)?;
        }
    }
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render_tui(frame: &mut Frame<'_>, app: &TuiApp) {
    let palette = Palette::new(app.cli.no_color);
    let area = frame.area();
    frame.render_widget(Block::default().style(palette.background), area);

    let shell = area.inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(shell);

    render_header(frame, vertical[0], palette);
    render_content(frame, vertical[1], app, palette);
    render_footer(frame, vertical[2], app, palette);

    match app.screen {
        Screen::SafeConfirmation => render_confirm_modal(
            frame,
            shell,
            palette,
            "Confirm safe cleanup",
            &[
                format!(
                    "Move {} selected Green items to managed Trash?",
                    app.selected_safe_count()
                ),
                "Press y to clean or n to cancel.".to_owned(),
            ],
        ),
        Screen::SessionConfirmation => render_confirm_modal(
            frame,
            shell,
            palette,
            "Confirm session cleanup",
            &[
                format!(
                    "Back up and move {} selected Yellow sessions to Trash?",
                    app.selected_session_count()
                ),
                "Press y to continue or n to cancel.".to_owned(),
            ],
        ),
        _ => {}
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, palette: Palette) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(38), Constraint::Length(48)])
        .split(area);
    let logo = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "VHAUL",
            palette.logo.add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled("Haul away your agent clutter.", palette.muted)),
        Line::from(Span::styled(
            "local-first / reversible / no telemetry",
            palette.accent,
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(palette.accent)
            .padding(Padding::left(2)),
    );
    frame.render_widget(logo, chunks[0]);

    let quick = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("enter", palette.accent.add_modifier(Modifier::BOLD)),
            Span::raw(" continue / open"),
        ]),
        Line::from(vec![
            Span::styled("space", palette.accent.add_modifier(Modifier::BOLD)),
            Span::raw(" toggle row"),
        ]),
        Line::from(vec![
            Span::styled("y/n", palette.accent.add_modifier(Modifier::BOLD)),
            Span::raw(" confirm popup"),
        ]),
    ])
    .style(palette.foreground)
    .block(
        Block::default()
            .title("Quick start")
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(palette.accent)
            .padding(Padding::horizontal(2)),
    );
    frame.render_widget(quick, chunks[1]);
}

fn render_content(frame: &mut Frame<'_>, area: Rect, app: &TuiApp, palette: Palette) {
    let title = screen_title(app.screen);
    let body = match app.screen {
        Screen::SafeConfirmation => app.render_safe_selection(),
        Screen::SessionConfirmation => app.render_session_browser(),
        _ => app.render(),
    };
    let panel = Paragraph::new(body)
        .style(palette.foreground)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(palette.border)
                .style(palette.panel)
                .padding(Padding::new(3, 3, 1, 1)),
        );
    frame.render_widget(panel, centered_panel(area));
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &TuiApp, palette: Palette) {
    let cwd = env::current_dir()
        .ok()
        .map_or_else(|| "cwd unknown".to_owned(), |path| display_path(&path));
    let version = env!("CARGO_PKG_VERSION");
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("cwd ", palette.accent.add_modifier(Modifier::BOLD)),
        Span::styled(truncate(cwd, 58), palette.muted),
        Span::raw("    "),
        Span::styled("v", palette.muted),
        Span::styled(version, palette.muted),
        Span::raw("    "),
        Span::styled(
            if app.cli.no_color {
                "no-color"
            } else {
                "color"
            },
            palette.muted,
        ),
    ]));
    frame.render_widget(footer, area);
}

fn render_confirm_modal(
    frame: &mut Frame<'_>,
    area: Rect,
    palette: Palette,
    title: &str,
    lines: &[String],
) {
    let popup = centered_rect(56, 9, area);
    frame.render_widget(Clear, popup);
    let text = Paragraph::new(vec![
        Line::from(Span::styled(
            lines.first().map_or("Confirm?", String::as_str),
            palette.foreground.add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            lines.get(1).map_or("Press y or n.", String::as_str),
            palette.muted,
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" y ", palette.confirm_button),
            Span::raw("  yes     "),
            Span::styled(" n ", palette.cancel_button),
            Span::raw("  no"),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(palette.accent)
            .style(palette.modal)
            .padding(Padding::new(2, 2, 1, 1)),
    );
    frame.render_widget(text, popup);
}

fn centered_panel(area: Rect) -> Rect {
    let width = area.width.min(104);
    let height = area.height.min(26);
    centered_rect(width, height, area)
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(area);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(horizontal[1]);
    vertical[1]
}

fn screen_title(screen: Screen) -> &'static str {
    match screen {
        Screen::Boot => "Discovery",
        Screen::AppSelection => "App selection",
        Screen::Analyze => "Analysis",
        Screen::SafeSelection | Screen::SafeConfirmation => "Safe cleanup",
        Screen::SafeExecution => "Cleaning safe items",
        Screen::SessionOverview => "Session data",
        Screen::SessionBrowser | Screen::SessionConfirmation => "Session browser",
        Screen::SessionExecution => "Cleaning sessions",
        Screen::FinalSummary => "Final summary",
    }
}

#[derive(Clone, Copy)]
struct Palette {
    background: Style,
    panel: Style,
    modal: Style,
    foreground: Style,
    muted: Style,
    accent: Style,
    border: Style,
    logo: Style,
    confirm_button: Style,
    cancel_button: Style,
}

impl Palette {
    fn new(no_color: bool) -> Self {
        if no_color {
            return Self {
                background: Style::default(),
                panel: Style::default(),
                modal: Style::default(),
                foreground: Style::default(),
                muted: Style::default(),
                accent: Style::default(),
                border: Style::default(),
                logo: Style::default(),
                confirm_button: Style::default().add_modifier(Modifier::REVERSED),
                cancel_button: Style::default().add_modifier(Modifier::REVERSED),
            };
        }

        Self {
            background: Style::default().bg(Color::Rgb(9, 24, 34)),
            panel: Style::default().bg(Color::Rgb(39, 37, 42)),
            modal: Style::default()
                .fg(Color::Rgb(232, 232, 232))
                .bg(Color::Rgb(22, 23, 24)),
            foreground: Style::default().fg(Color::Rgb(224, 226, 226)),
            muted: Style::default().fg(Color::Rgb(170, 168, 172)),
            accent: Style::default().fg(Color::Rgb(125, 221, 232)),
            border: Style::default().fg(Color::Rgb(70, 67, 74)),
            logo: Style::default().fg(Color::Rgb(190, 190, 188)),
            confirm_button: Style::default()
                .fg(Color::Rgb(19, 24, 26))
                .bg(Color::Rgb(178, 232, 128))
                .add_modifier(Modifier::BOLD),
            cancel_button: Style::default()
                .fg(Color::Rgb(232, 232, 232))
                .bg(Color::Rgb(82, 78, 86)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    Boot,
    AppSelection,
    Analyze,
    SafeSelection,
    SafeConfirmation,
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
    config: VibeHaulerConfig,
    ctx: PathContext,
    registry: AdapterRegistry,
    screen: Screen,
    app_rows: Vec<AppRow>,
    app_cursor: usize,
    inventory: Vec<InventoryItem>,
    sessions: Vec<AgentSession>,
    safe_selected: Vec<bool>,
    safe_cursor: usize,
    session_cursor: usize,
    session_browser_cursor: usize,
    session_browser_scroll: usize,
    browser_app: Option<AppId>,
    session_filter: String,
    session_filtering: bool,
    session_selected: BTreeSet<usize>,
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
        Ok(Self {
            cli,
            config,
            ctx,
            registry,
            screen: Screen::Boot,
            app_rows: Vec::new(),
            app_cursor: 0,
            inventory: Vec::new(),
            sessions: Vec::new(),
            safe_selected: Vec::new(),
            safe_cursor: 0,
            session_cursor: 0,
            session_browser_cursor: 0,
            session_browser_scroll: 0,
            browser_app: None,
            session_filter: String::new(),
            session_filtering: false,
            session_selected: BTreeSet::new(),
            safe_manifest: None,
            session_manifest: None,
            error: None,
            should_quit: false,
        })
    }

    fn discover(&mut self) -> anyhow::Result<()> {
        let mut instances = self.registry.detect_all(&self.ctx)?;
        instances.extend(user_root_instances(self.cli.roots.as_ref(), self.ctx.os)?);
        instances.extend(config_root_instances(&self.config, self.ctx.os));
        instances.sort_by(|left, right| {
            left.app
                .cmp(&right.app)
                .then_with(|| left.root.cmp(&right.root))
        });
        instances.dedup_by(|left, right| left.app == right.app && left.root == right.root);

        self.app_rows = instances
            .into_iter()
            .filter(|instance| self.config.app_enabled(&instance.app))
            .map(|instance| AppRow {
                selected: true,
                selectable: true,
                instance,
            })
            .collect::<Vec<_>>();
        self.screen = Screen::AppSelection;
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if is_ctrl_c(key) {
            self.should_quit = true;
            return Ok(());
        }
        if self.screen == Screen::SessionBrowser && self.session_filtering {
            return self.handle_session_browser(key);
        }
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

        self.error = None;
        match self.screen {
            Screen::Boot | Screen::Analyze | Screen::SafeExecution | Screen::SessionExecution => {
                Ok(())
            }
            Screen::AppSelection => self.handle_app_selection(key),
            Screen::SafeSelection => self.handle_safe_selection(key),
            Screen::SafeConfirmation => self.handle_safe_confirmation(key),
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
                if self.selected_safe_count() == 0 {
                    self.screen = Screen::SessionOverview;
                } else {
                    self.screen = Screen::SafeConfirmation;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_safe_confirmation(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Char('y' | 'Y') => {
                self.screen = Screen::SafeExecution;
                self.execute_safe_cleanup()?;
                self.screen = Screen::SessionOverview;
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.screen = Screen::SafeSelection;
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
                    self.open_session_browser(app);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_browser(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if self.session_filtering {
            return self.handle_session_filter_key(key);
        }
        match key.code {
            KeyCode::Esc => self.screen = Screen::SessionOverview,
            KeyCode::Down => self.move_session_browser_cursor(1),
            KeyCode::Up => self.move_session_browser_cursor(-1),
            KeyCode::PageDown => self.page_session_browser(1),
            KeyCode::PageUp => self.page_session_browser(-1),
            KeyCode::Home => self.set_session_browser_cursor(0),
            KeyCode::End => {
                let len = self.visible_session_indices().len();
                self.set_session_browser_cursor(len.saturating_sub(1));
            }
            KeyCode::Char('/') => {
                self.session_filtering = true;
                self.error = None;
            }
            KeyCode::Char('c' | 'C') => {
                self.session_filter.clear();
                self.reset_session_browser_position();
            }
            KeyCode::Char('a') => {
                let cleanable = self
                    .visible_session_indices()
                    .into_iter()
                    .filter(|index| self.is_session_cleanable(*index))
                    .collect::<Vec<_>>();
                if cleanable.is_empty() {
                    self.error =
                        Some("No selectable Yellow sessions in the current view.".to_owned());
                }
                for index in cleanable {
                    self.session_selected.insert(index);
                }
            }
            KeyCode::Char('n' | 'N') => self.visible_session_indices().iter().for_each(|index| {
                self.session_selected.remove(index);
            }),
            KeyCode::Char(' ') => {
                if let Some(index) = self.current_browser_session_index()
                    && self.is_session_cleanable(index)
                    && !self.session_selected.remove(&index)
                {
                    self.session_selected.insert(index);
                } else if let Some(index) = self.current_browser_session_index()
                    && !self.is_session_cleanable(index)
                {
                    self.error = Some(
                        "This session is report-only; DB cleanup is not supported yet.".to_owned(),
                    );
                }
            }
            KeyCode::Enter => {
                if self.selected_session_count() > 0 {
                    self.screen = Screen::SessionConfirmation;
                } else if self
                    .current_browser_session_index()
                    .is_some_and(|index| !self.is_session_cleanable(index))
                {
                    self.error = Some(
                        "This app is report-only for now; database cleanup is disabled.".to_owned(),
                    );
                } else {
                    self.error = Some("Select at least one session before cleanup.".to_owned());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_filter_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                self.session_filtering = false;
                self.clamp_session_browser_position();
            }
            KeyCode::Backspace => {
                self.session_filter.pop();
                self.reset_session_browser_position();
            }
            KeyCode::Char(value) => {
                self.session_filter.push(value);
                self.reset_session_browser_position();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_confirmation(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Char('y' | 'Y') => {
                self.screen = Screen::SessionExecution;
                self.execute_session_cleanup()?;
                self.screen = Screen::FinalSummary;
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.screen = Screen::SessionBrowser;
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
            .filter_map(|index| {
                self.sessions
                    .get(*index)
                    .filter(|session| session.risk == RiskLevel::Yellow)
                    .cloned()
            })
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
            .filter_map(|(index, session)| {
                (&session.app == app && self.session_matches_filter(session)).then_some(index)
            })
            .collect()
    }

    fn session_matches_filter(&self, session: &AgentSession) -> bool {
        let query = self.session_filter.trim();
        if query.is_empty() {
            return true;
        }
        let query = query.to_lowercase();
        session
            .title
            .as_deref()
            .is_some_and(|value| value.to_lowercase().contains(&query))
            || session
                .cwd
                .as_ref()
                .is_some_and(|value| display_path(value).to_lowercase().contains(&query))
            || session
                .updated_at
                .as_deref()
                .is_some_and(|value| value.to_lowercase().contains(&query))
            || session
                .preview
                .as_deref()
                .is_some_and(|value| value.to_lowercase().contains(&query))
    }

    fn open_session_browser(&mut self, app: AppId) {
        self.browser_app = Some(app);
        self.session_browser_cursor = 0;
        self.session_browser_scroll = 0;
        self.session_filter.clear();
        self.session_filtering = false;
        self.screen = Screen::SessionBrowser;
    }

    fn current_browser_session_index(&self) -> Option<usize> {
        self.visible_session_indices()
            .get(self.session_browser_cursor)
            .copied()
    }

    fn reset_session_browser_position(&mut self) {
        self.session_browser_cursor = 0;
        self.session_browser_scroll = 0;
    }

    fn set_session_browser_cursor(&mut self, cursor: usize) {
        let len = self.visible_session_indices().len();
        self.session_browser_cursor = if len == 0 { 0 } else { cursor.min(len - 1) };
        self.sync_session_browser_scroll(len);
    }

    fn move_session_browser_cursor(&mut self, delta: isize) {
        let len = self.visible_session_indices().len();
        if len == 0 {
            self.reset_session_browser_position();
            return;
        }
        let cursor = if delta.is_negative() {
            self.session_browser_cursor
                .saturating_sub(delta.unsigned_abs())
        } else {
            self.session_browser_cursor
                .saturating_add(delta.unsigned_abs())
                .min(len - 1)
        };
        self.session_browser_cursor = cursor;
        self.sync_session_browser_scroll(len);
    }

    fn page_session_browser(&mut self, direction: isize) {
        let len = self.visible_session_indices().len();
        if len == 0 {
            self.reset_session_browser_position();
            return;
        }
        let max_scroll = len.saturating_sub(SESSION_BROWSER_PAGE_SIZE);
        if direction.is_negative() {
            self.session_browser_scroll = self
                .session_browser_scroll
                .saturating_sub(SESSION_BROWSER_PAGE_SIZE);
        } else {
            self.session_browser_scroll = self
                .session_browser_scroll
                .saturating_add(SESSION_BROWSER_PAGE_SIZE)
                .min(max_scroll);
        }
        self.session_browser_cursor = self.session_browser_scroll.min(len - 1);
    }

    fn clamp_session_browser_position(&mut self) {
        let len = self.visible_session_indices().len();
        if len == 0 {
            self.reset_session_browser_position();
            return;
        }
        self.session_browser_cursor = self.session_browser_cursor.min(len - 1);
        self.sync_session_browser_scroll(len);
    }

    fn sync_session_browser_scroll(&mut self, len: usize) {
        if len == 0 {
            self.session_browser_scroll = 0;
            return;
        }
        if self.session_browser_cursor < self.session_browser_scroll {
            self.session_browser_scroll = self.session_browser_cursor;
        }
        let page_end = self
            .session_browser_scroll
            .saturating_add(SESSION_BROWSER_PAGE_SIZE);
        if self.session_browser_cursor >= page_end {
            self.session_browser_scroll =
                self.session_browser_cursor + 1 - SESSION_BROWSER_PAGE_SIZE;
        }
        let max_scroll = len.saturating_sub(SESSION_BROWSER_PAGE_SIZE);
        self.session_browser_scroll = self.session_browser_scroll.min(max_scroll);
    }

    fn selected_session_count(&self) -> usize {
        self.session_selected.len()
    }

    fn is_session_cleanable(&self, index: usize) -> bool {
        self.sessions
            .get(index)
            .is_some_and(|session| session.risk == RiskLevel::Yellow)
    }

    fn selected_safe_count(&self) -> usize {
        self.safe_selected.iter().filter(|value| **value).count()
    }

    fn render(&self) -> String {
        let mut output = match self.screen {
            Screen::Boot => self.render_boot(),
            Screen::AppSelection => self.render_app_selection(),
            Screen::Analyze => self.render_analysis(),
            Screen::SafeSelection => self.render_safe_selection(),
            Screen::SafeConfirmation => self.render_safe_confirmation(),
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

    fn render_boot(&self) -> String {
        "+-- VibeHauler -------------------------------- Local cleanup ----+\n| Finding local agent data...                                     |\n|                                                                |\n| Discovery is read-only. Press q after the next screen to quit.  |\n+-----------------------------------------------------------------+".to_owned()
    }

    fn render_app_selection(&self) -> String {
        let mut lines = vec![
            "+-- VibeHauler ----------------------------- Local-only cleanup --+".to_owned(),
            "| Select apps to inspect                                          |".to_owned(),
            "| All detected apps are selected by default.                      |".to_owned(),
            "+----+--------------------+------------------------+--------------+".to_owned(),
            "|    | App                | Data root              | Status       |".to_owned(),
            "+----+--------------------+------------------------+--------------+".to_owned(),
        ];
        if self.app_rows.is_empty() {
            lines.push(
                "|    | No supported apps found in this home                       |".to_owned(),
            );
        } else {
            for (index, row) in self.app_rows.iter().enumerate() {
                lines.push(format!(
                    "| {}  | [{}] {:14} | {:22} | {:12} |",
                    if index == self.app_cursor { ">" } else { " " },
                    if row.selected { "x" } else { " " },
                    truncate(row.instance.display_name.as_str(), 14),
                    truncate(display_path(&row.instance.root), 22),
                    "found"
                ));
            }
        }
        lines.extend([
            "+----+--------------------+------------------------+--------------+".to_owned(),
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
                    self.selected_safe_count()
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
            "| Enter Review cleanup - Space Toggle - q Quit                    |".to_owned(),
            "+-----------------------------------------------------------------+".to_owned(),
        ]);
        lines.join("\n")
    }

    fn render_safe_confirmation(&self) -> String {
        format!(
            "+-- Confirm safe cleanup -----------------------------------------+\n| Move {} selected Green items to managed Trash?                  |\n| Press y to clean or n to cancel.                                |\n+-----------------------------------------------------------------+",
            self.selected_safe_count()
        )
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
                let size = aggregate_session_size(&app_sessions);
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
        let all_count = self.browser_app.as_ref().map_or(0, |app| {
            self.sessions
                .iter()
                .filter(|session| &session.app == app)
                .count()
        });
        let cleanable_count = visible
            .iter()
            .filter(|index| self.is_session_cleanable(**index))
            .count();
        let selected_visible = visible
            .iter()
            .filter(|index| self.session_selected.contains(index))
            .count();
        let title = self
            .browser_app
            .as_ref()
            .map_or("Sessions".to_owned(), |app| {
                format!("Sessions / {}", app.display_name())
            });
        let cursor = self
            .session_browser_cursor
            .min(visible.len().saturating_sub(1));
        let scroll = normalized_scroll(
            cursor,
            self.session_browser_scroll,
            visible.len(),
            SESSION_BROWSER_PAGE_SIZE,
        );
        let end = scroll
            .saturating_add(SESSION_BROWSER_PAGE_SIZE)
            .min(visible.len());
        let range = if visible.is_empty() {
            "0-0 / 0".to_owned()
        } else {
            format!("{}-{} / {}", scroll + 1, end, visible.len())
        };
        let filter = if self.session_filter.trim().is_empty() {
            "filter: none".to_owned()
        } else {
            format!("filter: {}", self.session_filter)
        };
        let mode = if self.session_filtering {
            "filter mode"
        } else {
            "browse mode"
        };
        let mut lines = vec![
            format!("+-- {:<86}+", truncate(&title, 86)),
            format!(
                "| {:<91} |",
                truncate(
                    format!(
                        "{} of {} sessions - {} selected here - {} selected total - {} selectable",
                        visible.len(),
                        all_count,
                        selected_visible,
                        self.selected_session_count(),
                        cleanable_count
                    ),
                    91
                )
            ),
            format!(
                "| {:<91} |",
                truncate(format!("{range} - {mode} - {filter}"), 91)
            ),
            "+----+------------------+--------------------------------+------+--------------------+".to_owned(),
            "|    | Updated          | Directory                      | Size | Title              |".to_owned(),
            "+----+------------------+--------------------------------+------+--------------------+".to_owned(),
        ];
        self.push_session_browser_rows(&mut lines, &visible, scroll, end, cursor);
        self.push_session_browser_footer(&mut lines, &visible, cursor);
        lines.join("\n")
    }

    fn push_session_browser_rows(
        &self,
        lines: &mut Vec<String>,
        visible: &[usize],
        scroll: usize,
        end: usize,
        cursor: usize,
    ) {
        if visible.is_empty() {
            lines.push(
                "|    | No sessions match the current filter                                  |"
                    .to_owned(),
            );
            return;
        }
        for (row, session_index) in visible[scroll..end].iter().enumerate() {
            let absolute_row = scroll + row;
            let session = &self.sessions[*session_index];
            lines.push(format!(
                "| {}  | [{}] {:12} | {:30} | {:4} | {:18} |",
                if absolute_row == cursor { ">" } else { " " },
                self.session_marker(*session_index),
                truncate(session.updated_at.as_deref().unwrap_or("unknown"), 12),
                truncate(
                    session
                        .cwd
                        .as_ref()
                        .map_or("unknown".to_owned(), |path| display_path(path)),
                    30
                ),
                truncate(session_size_label(session), 4),
                truncate(session.title.as_deref().unwrap_or("untitled"), 18),
            ));
        }
    }

    fn session_marker(&self, session_index: usize) -> &'static str {
        if !self.is_session_cleanable(session_index) {
            "-"
        } else if self.session_selected.contains(&session_index) {
            "x"
        } else {
            " "
        }
    }

    fn push_session_browser_footer(
        &self,
        lines: &mut Vec<String>,
        visible: &[usize],
        cursor: usize,
    ) {
        let preview = visible
            .get(cursor)
            .and_then(|index| self.sessions.get(*index))
            .and_then(|session| session.preview.as_deref())
            .unwrap_or("");
        let status = self.error.as_deref().unwrap_or(if self.session_filtering {
            "Type to filter - Backspace delete - Enter/Esc finish"
        } else {
            "Up/Down move - PgUp/PgDn page - / filter - c clear - a/n all/none - Esc back"
        });
        lines.extend([
            "+----+------------------+--------------------------------+------+--------------------+".to_owned(),
            format!("| Preview: {:<83} |", truncate(preview, 83)),
            format!("| {:<91} |", truncate(status, 91)),
            "+---------------------------------------------------------------------------------------------+"
                .to_owned(),
        ]);
    }

    fn render_session_confirmation(&self) -> String {
        format!(
            "+-- Confirm session cleanup --------------------------------------+\n| Back up and move {} selected Yellow sessions to Trash?          |\n| Press y to continue or n to cancel.                             |\n+-----------------------------------------------------------------+",
            self.selected_session_count()
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

fn is_ctrl_c(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c' | 'C')) && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn normalized_scroll(cursor: usize, scroll: usize, len: usize, page_size: usize) -> usize {
    if len == 0 || page_size == 0 {
        return 0;
    }
    let mut scroll = scroll.min(len.saturating_sub(1));
    if cursor < scroll {
        scroll = cursor;
    } else if cursor >= scroll.saturating_add(page_size) {
        scroll = cursor + 1 - page_size;
    }
    scroll.min(len.saturating_sub(page_size))
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

fn aggregate_session_size(sessions: &[&AgentSession]) -> u64 {
    let mut database_sources = BTreeSet::new();
    let mut total = 0_u64;
    for session in sessions {
        match &session.source {
            SessionSource::Database(path) => {
                if database_sources.insert(path.clone()) {
                    total = total
                        .saturating_add(sqlite_backing_size(path).unwrap_or(session.size_bytes));
                }
            }
            _ => {
                total = total.saturating_add(session.size_bytes);
            }
        }
    }
    total
}

fn session_size_label(session: &AgentSession) -> String {
    if matches!(session.source, SessionSource::Database(_)) {
        "db".to_owned()
    } else {
        human_bytes(session.size_bytes)
    }
}

fn sqlite_backing_size(path: &Path) -> Option<u64> {
    let mut total = metadata_len(path)?;
    for suffix in ["-wal", "-shm"] {
        total = total.saturating_add(metadata_len(&path_with_suffix(path, suffix)).unwrap_or(0));
    }
    Some(total)
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = OsString::from(path.as_os_str());
    value.push(suffix);
    PathBuf::from(value)
}

fn metadata_len(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|metadata| metadata.len())
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn database_backed_sessions_count_source_size_once() {
        let temp = tempdir().expect("tempdir");
        let db = temp.path().join("agent.db");
        fs::write(&db, [0_u8; 10]).expect("write db");
        fs::write(path_with_suffix(&db, "-wal"), [0_u8; 5]).expect("write wal");
        fs::write(path_with_suffix(&db, "-shm"), [0_u8; 3]).expect("write shm");

        let first = test_session("first", SessionSource::Database(db.clone()), 10);
        let second = test_session("second", SessionSource::Database(db), 10);
        let file = test_session(
            "file",
            SessionSource::File(temp.path().join("session.jsonl")),
            7,
        );

        let sessions = vec![&first, &second, &file];
        assert_eq!(aggregate_session_size(&sessions), 25);
        assert_eq!(session_size_label(&first), "db");
    }

    #[test]
    fn session_browser_paginates_filters_and_blocks_report_only_selection() {
        let mut app = test_app();
        let db = PathBuf::from("/tmp/agent.db");
        app.sessions = (0..25)
            .map(|index| {
                let mut session = test_session(
                    &format!("session-{index}"),
                    SessionSource::Database(db.clone()),
                    10,
                );
                session.title = Some(if index == 18 {
                    "needle match".to_owned()
                } else {
                    format!("row {index}")
                });
                session.cwd = Some(PathBuf::from(format!("/work/project-{index}")));
                session.updated_at = Some(format!("2026-05-{:02}T00:00:00Z", index + 1));
                session
            })
            .collect();

        app.open_session_browser(AppId::DeepChat);
        let first_page = app.render_session_browser();
        assert!(first_page.contains("1-11 / 25"));
        assert!(first_page.contains("| >  | [-]"));

        app.page_session_browser(1);
        let second_page = app.render_session_browser();
        assert!(second_page.contains("12-22 / 25"));

        app.session_filter = "needle".to_owned();
        app.reset_session_browser_position();
        let filtered = app.render_session_browser();
        assert!(filtered.contains("1-1 / 1"));
        assert!(filtered.contains("needle match"));

        app.handle_session_browser(KeyEvent::from(KeyCode::Char('a')))
            .expect("select all");
        assert_eq!(app.selected_session_count(), 0);
        assert!(
            app.error
                .as_deref()
                .is_some_and(|value| value.contains("No selectable"))
        );
    }

    #[test]
    fn ctrl_c_quits_even_while_filtering() {
        let mut app = test_app();
        app.screen = Screen::SessionBrowser;
        app.session_filtering = true;
        app.session_filter = "abc".to_owned();

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
            .expect("handle ctrl-c");

        assert!(app.should_quit);
        assert_eq!(app.session_filter, "abc");
    }

    fn test_app() -> TuiApp {
        TuiApp {
            cli: Cli {
                config: None,
                no_color: true,
                portable_root: None,
                roots: None,
            },
            config: VibeHaulerConfig::default(),
            ctx: PathContext::for_portable_root("/tmp/vhaul-test", OsKind::MacOS),
            registry: AdapterRegistry::v01(),
            screen: Screen::Boot,
            app_rows: Vec::new(),
            app_cursor: 0,
            inventory: Vec::new(),
            sessions: Vec::new(),
            safe_selected: Vec::new(),
            safe_cursor: 0,
            session_cursor: 0,
            session_browser_cursor: 0,
            session_browser_scroll: 0,
            browser_app: None,
            session_filter: String::new(),
            session_filtering: false,
            session_selected: BTreeSet::new(),
            safe_manifest: None,
            session_manifest: None,
            error: None,
            should_quit: false,
        }
    }

    fn test_session(id: &str, source: SessionSource, size_bytes: u64) -> AgentSession {
        AgentSession {
            id: id.to_owned(),
            app: AppId::DeepChat,
            title: None,
            cwd: None,
            started_at: None,
            updated_at: None,
            turns: None,
            tokens: None,
            files: Vec::new(),
            size_bytes,
            preview: None,
            preview_hash: None,
            risk: RiskLevel::Black,
            source,
            parser: "test".to_owned(),
        }
    }
}

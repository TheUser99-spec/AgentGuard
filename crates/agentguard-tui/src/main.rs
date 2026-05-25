//! AgentGuard TUI — Dashboard interactivo ratatui.
//!
//! Muestra:
//!   - Estado del daemon
//!   - Agentes activos en tiempo real
//!   - Proyectos vigilados con conteos
//!   - Últimos eventos de auditoría

use agentguard_ipc::{ActiveAgent, DaemonStatus, IpcClient, ProjectInfo};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs};
use ratatui::Frame;
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut app = App::new();
    enable_raw_mode()?;
    io::stdout().execute(crossterm::terminal::EnterAlternateScreen)?;

    let result = app.run().await;

    disable_raw_mode()?;
    io::stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;

    result
}

struct App {
    client: IpcClient,
    tabs: Vec<&'static str>,
    active_tab: usize,
    status: Option<DaemonStatus>,
    running: bool,
    error: Option<String>,
}

impl App {
    fn new() -> Self {
        Self {
            client: IpcClient::new(),
            tabs: vec!["Status", "Agents", "Projects", "Events"],
            active_tab: 0,
            status: None,
            running: true,
            error: None,
        }
    }

    async fn run(&mut self) -> io::Result<()> {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;

        let tick = Duration::from_millis(800);
        let mut last_tick = tokio::time::Instant::now() - tick;

        while self.running {
            terminal.draw(|f| self.draw(f))?;

            let timeout = Duration::from_millis(100);
            if event::poll(timeout).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            if last_tick.elapsed() >= tick {
                self.fetch_status().await;
                last_tick = tokio::time::Instant::now();
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Tab | KeyCode::Right => {
                self.active_tab = (self.active_tab + 1) % self.tabs.len();
            }
            KeyCode::BackTab | KeyCode::Left => {
                if self.active_tab == 0 {
                    self.active_tab = self.tabs.len() - 1;
                } else {
                    self.active_tab -= 1;
                }
            }
            KeyCode::Char('1') => self.active_tab = 0,
            KeyCode::Char('2') => self.active_tab = 1,
            KeyCode::Char('3') => self.active_tab = 2,
            KeyCode::Char('4') => self.active_tab = 3,
            _ => {}
        }
    }

    async fn fetch_status(&mut self) {
        match self.client.get_status().await {
            Ok(s) => {
                self.status = Some(s);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Daemon: {e}"));
            }
        }
    }

    fn draw(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(f.area());

        self.draw_header(f, chunks[0]);
        self.draw_tabs(f, chunks[1]);
        self.draw_body(f, chunks[2]);
        self.draw_footer(f, chunks[3]);
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let title = Line::from(vec![
            Span::styled(
                "AgentGuard",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" — "),
            Span::styled(
                "OS-level security for AI agents",
                Style::default().fg(Color::Gray),
            ),
        ]);
        let p =
            Paragraph::new(Text::from(vec![title])).block(Block::default().borders(Borders::NONE));
        f.render_widget(p, area);
    }

    fn draw_tabs(&self, f: &mut Frame, area: Rect) {
        let titles: Vec<Line> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if i == self.active_tab {
                    Line::from(Span::styled(
                        format!(" {t} "),
                        Style::default().fg(Color::Black).bg(Color::Green),
                    ))
                } else {
                    Line::from(Span::styled(
                        format!(" {t} "),
                        Style::default().fg(Color::Gray),
                    ))
                }
            })
            .collect();

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::BOTTOM))
            .select(self.active_tab)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(tabs, area);
    }

    fn draw_body(&self, f: &mut Frame, area: Rect) {
        match self.active_tab {
            0 => self.draw_status_tab(f, area),
            1 => self.draw_agents_tab(f, area),
            2 => self.draw_projects_tab(f, area),
            3 => self.draw_events_tab(f, area),
            _ => {}
        }
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" quit  "),
            Span::styled(
                " 1-4 ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" tabs  "),
            Span::styled(
                " Tab ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" next"),
        ];

        if let Some(s) = &self.status {
            spans.push(Span::raw("  |  "));
            spans.push(Span::styled(
                format!(" {} projects ", s.projects.len()),
                Style::default().fg(Color::Green),
            ));
            spans.push(Span::styled(
                format!(" {} agents ", s.active_agents.len()),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(
                format!(" {} blocks today ", s.blocks_today),
                Style::default().fg(Color::Red),
            ));
        }

        if let Some(e) = &self.error {
            spans.push(Span::raw("  |  "));
            spans.push(Span::styled(e, Style::default().fg(Color::Red)));
        }

        let p = Paragraph::new(Line::from(spans));
        f.render_widget(p, area);
    }

    // ── Status Tab ───────────────────────────────────────────────────────

    fn draw_status_tab(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(7), Constraint::Min(0)])
            .split(area);

        if let Some(s) = &self.status {
            let lines = vec![
                Line::from(vec![
                    Span::raw("  Daemon: "),
                    Span::styled(
                        "RUNNING",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![Span::raw("  Version: "), Span::raw(&s.version)]),
                Line::from(vec![
                    Span::raw("  Projects: "),
                    Span::styled(
                        format!("{}", s.projects.len()),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw("  |  Agents active: "),
                    Span::styled(
                        format!("{}", s.active_agents.len()),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  Events today: "),
                    Span::styled(
                        format!("{}", s.events_today),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw("  |  Blocks: "),
                    Span::styled(
                        format!("{}", s.blocks_today),
                        Style::default().fg(Color::Red),
                    ),
                ]),
            ];
            let p = Paragraph::new(Text::from(lines)).block(
                Block::default()
                    .title(" Daemon Status ")
                    .borders(Borders::ALL),
            );
            f.render_widget(p, chunks[0]);
        } else {
            let p = Paragraph::new("  Connecting to daemon...").block(
                Block::default()
                    .title(" Daemon Status ")
                    .borders(Borders::ALL),
            );
            f.render_widget(p, chunks[0]);
        }

        let help = vec![
            Line::from(Span::styled(
                "  Keybindings",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  q / Esc   Quit"),
            Line::from("  Tab       Next tab"),
            Line::from("  1-4       Jump to tab"),
            Line::from(""),
            Line::from(Span::styled(
                "  Commands",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  agentguard init               Initialize a project"),
            Line::from("  agentguard status             Show daemon status"),
            Line::from("  agentguard project check ...  Dry-run a file access"),
            Line::from("  agentguard daemon start/stop  Control the daemon"),
        ];
        let p = Paragraph::new(Text::from(help))
            .block(Block::default().title(" Help ").borders(Borders::ALL));
        f.render_widget(p, chunks[1]);
    }

    // ── Agents Tab ───────────────────────────────────────────────────────

    fn draw_agents_tab(&self, f: &mut Frame, area: Rect) {
        if let Some(s) = &self.status {
            if s.active_agents.is_empty() {
                let p = Paragraph::new("  No active AI agents detected.").block(
                    Block::default()
                        .title(" Active Agents ")
                        .borders(Borders::ALL),
                );
                f.render_widget(p, area);
                return;
            }

            let items: Vec<ListItem> = s.active_agents.iter().map(|a| agent_list_item(a)).collect();

            let list = List::new(items).block(
                Block::default()
                    .title(format!(" Active Agents ({}) ", s.active_agents.len()))
                    .borders(Borders::ALL),
            );
            f.render_widget(list, area);
        } else {
            let p = Paragraph::new("  Connecting...").block(
                Block::default()
                    .title(" Active Agents ")
                    .borders(Borders::ALL),
            );
            f.render_widget(p, area);
        }
    }

    // ── Projects Tab ─────────────────────────────────────────────────────

    fn draw_projects_tab(&self, f: &mut Frame, area: Rect) {
        if let Some(s) = &self.status {
            if s.projects.is_empty() {
                let p = Paragraph::new("  No projects registered.\n  Run: agentguard init").block(
                    Block::default()
                        .title(" Watched Projects ")
                        .borders(Borders::ALL),
                );
                f.render_widget(p, area);
                return;
            }

            let items: Vec<ListItem> = s.projects.iter().map(|p| project_list_item(p)).collect();

            let list = List::new(items).block(
                Block::default()
                    .title(format!(" Watched Projects ({}) ", s.projects.len()))
                    .borders(Borders::ALL),
            );
            f.render_widget(list, area);
        } else {
            let p = Paragraph::new("  Connecting...").block(
                Block::default()
                    .title(" Watched Projects ")
                    .borders(Borders::ALL),
            );
            f.render_widget(p, area);
        }
    }

    // ── Events Tab ───────────────────────────────────────────────────────

    fn draw_events_tab(&self, f: &mut Frame, area: Rect) {
        if let Some(s) = &self.status {
            if s.recent_events.is_empty() {
                let lines = vec![
                    Line::from(vec![
                        Span::raw("  Events today: "),
                        Span::styled(
                            format!("{}", s.events_today),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw("  |  Blocks: "),
                        Span::styled(
                            format!("{}", s.blocks_today),
                            Style::default().fg(Color::Red),
                        ),
                    ]),
                    Line::from(""),
                    Line::from("  No recent audit events."),
                ];
                let p = Paragraph::new(Text::from(lines)).block(
                    Block::default()
                        .title(" Audit Events ")
                        .borders(Borders::ALL),
                );
                f.render_widget(p, area);
                return;
            }

            let items: Vec<ListItem> = s.recent_events.iter().map(|e| event_list_item(e)).collect();

            let list = List::new(items).block(
                Block::default()
                    .title(format!(" Audit Events ({}) ", s.recent_events.len()))
                    .borders(Borders::ALL),
            );
            f.render_widget(list, area);
        } else {
            let p = Paragraph::new("  Connecting...").block(
                Block::default()
                    .title(" Audit Events ")
                    .borders(Borders::ALL),
            );
            f.render_widget(p, area);
        }
    }
}

fn agent_list_item(a: &ActiveAgent) -> ListItem<'_> {
    let color = match a.label {
        agentguard_core::AgentLabel::Definite => Color::Red,
        agentguard_core::AgentLabel::Probable => Color::Yellow,
        agentguard_core::AgentLabel::Inherited => Color::Magenta,
        agentguard_core::AgentLabel::Human => Color::Gray,
    };

    let workspace = a
        .workspace
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "--".into());

    ListItem::new(Line::from(vec![
        Span::styled(
            format!("  {:8} ", a.image_name),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" PID:{:<6} ", a.pid),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(format!("{:?} ", a.label), Style::default().fg(color)),
        Span::styled(workspace, Style::default().fg(Color::Gray)),
    ]))
}

fn event_list_item(e: &agentguard_ipc::AuditEventView) -> ListItem<'_> {
    let color = if e.decision == "deny" {
        Color::Red
    } else if e.decision == "ask" {
        Color::Yellow
    } else {
        Color::Green
    };

    let ts = chrono::DateTime::from_timestamp(e.timestamp, 0)
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "--".into());

    ListItem::new(Line::from(vec![
        Span::styled(format!(" {} ", ts), Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:>6} ", e.decision.to_uppercase()),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", e.agent_label),
            Style::default().fg(Color::Magenta),
        ),
        Span::raw(format!("PID:{} ", e.agent_pid)),
        Span::raw(format!("{} ", e.operation)),
        Span::styled(
            truncate_path(&e.file_path, 40),
            Style::default().fg(Color::Cyan),
        ),
    ]))
}

fn truncate_path(path: &str, max: usize) -> String {
    if path.len() <= max {
        path.to_string()
    } else {
        format!("...{}", &path[path.len().saturating_sub(max - 3)..])
    }
}

fn project_list_item(p: &ProjectInfo) -> ListItem<'_> {
    let counts = format!(
        "deny:{} ask:{} write:{} read:{}",
        p.deny_count, p.ask_count, p.write_count, p.read_count
    );

    ListItem::new(Line::from(vec![
        Span::styled(
            format!("  {} ", p.path.display()),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(counts, Style::default().fg(Color::Gray)),
    ]))
}

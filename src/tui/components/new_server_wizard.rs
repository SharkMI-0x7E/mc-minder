// New Server Wizard — multi-step server creation (Fabric/Paper/Vanilla).
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::tui::component::Component;
use crate::tui::state::{AppState, Language, MessageType};
use crate::tui::action::Action;
use crate::tui::widgets::centered_rect::centered_rect;
use crate::core_download::CoreType;

pub struct NewServerWizard {
    pub step: u8,
    pub selected: usize,
    pub core_types: Vec<CoreType>,
    pub versions: Vec<String>,
    pub language: Language,
}

impl NewServerWizard {
    pub fn new(language: Language) -> Self {
        Self { step: 0, selected: 0, core_types: CoreType::all(), versions: Vec::new(), language }
    }
}

impl Component for NewServerWizard {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        let max = match self.step {
            0 => self.core_types.len().saturating_sub(1),
            1 => self.versions.len().saturating_sub(1),
            _ => 0,
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.step = 0; Action::Navigate(AppState::MainMenu) }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => { self.selected = self.selected.saturating_sub(1); Action::Noop }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => { if self.selected < max { self.selected += 1; } Action::Noop }
            KeyCode::Enter => match self.step {
                0 => {
                    let ct = self.core_types[self.selected].clone();
                    let rt = tokio::runtime::Handle::current();
                    self.versions = match ct {
                        CoreType::Fabric => rt.block_on(crate::core_download::fetch_fabric_game_versions()).unwrap_or_else(|_| vec!["1.21.1".into(), "1.20.1".into()]),
                        CoreType::Vanilla => rt.block_on(crate::core_download::fetch_vanilla_versions()).unwrap_or_else(|_| vec!["1.21.1".into(), "1.20.1".into()]),
                        CoreType::Paper => rt.block_on(crate::core_download::fetch_paper_versions()).unwrap_or_else(|_| vec!["1.21".into(), "1.20".into()]),
                    };
                    self.versions.truncate(20);
                    self.step = 1; self.selected = 0;
                    Action::Noop
                }
                1 => {
                    let ct = self.core_types[0].clone();
                    let ver = self.versions[self.selected].clone();
                    let dir = std::env::current_dir().unwrap_or_default();
                    let rt = tokio::runtime::Handle::current();
                    let result = match ct {
                        CoreType::Fabric => {
                            let loader = rt.block_on(crate::core_download::fetch_fabric_loader(&ver)).unwrap_or_else(|_| "0.17.2".into());
                            rt.block_on(crate::core_download::download_fabric_server(&ver, &loader, &dir))
                        }
                        CoreType::Vanilla => rt.block_on(crate::core_download::download_vanilla_server(&ver, &dir)),
                        CoreType::Paper => rt.block_on(crate::core_download::download_paper_server(&ver, &dir)),
                    };
                    match result {
                        Ok(_) => Action::ShowMessage("Downloaded!".into(), MessageType::Success),
                        Err(e) => Action::ShowMessage(format!("Failed: {}", e), MessageType::Warning),
                    }
                }
                _ => Action::Noop,
            },
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let cn = matches!(self.language, Language::Chinese);
        let items: Vec<ListItem> = match self.step {
            0 => self.core_types.iter().map(|ct| ListItem::new(Span::raw(if cn { ct.display_name_cn() } else { ct.display_name() }))).collect(),
            1 => self.versions.iter().map(|v| ListItem::new(Span::raw(v.clone()))).collect(),
            _ => vec![ListItem::new(Span::raw(if cn { "下载中..." } else { "Downloading..." }))],
        };
        let mut state = ListState::default(); state.select(Some(self.selected));
        let title = match self.step {
            0 => if cn { "新建服务器 — 选择核心类型" } else { "New Server — Select Core Type" },
            1 => if cn { "选择 Minecraft 版本" } else { "Select Minecraft Version" },
            _ => if cn { "正在下载..." } else { "Downloading..." },
        };
        let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
        let popup = centered_rect(55, 30, area);
        frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), popup);
        frame.render_stateful_widget(list, popup, &mut state);
    }
}

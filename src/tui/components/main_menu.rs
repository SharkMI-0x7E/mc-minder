// Main Menu — core navigation hub with 6 categories and 4 sub-menus.
// The brain of the TUI. Handles main menu + Server/Monitor/Config/Advanced sub-menus.
use std::collections::VecDeque;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::tui::component::Component;
use crate::tui::state::{AppState, Language, ConfirmAction, LogType, MessageType};
use crate::tui::action::Action;
use crate::tui::widgets::centered_rect::centered_rect;
use crate::api::McStatusSnapshot;

#[derive(Clone, Copy, PartialEq)]
enum MenuMode { Main, SubServer, SubMonitor, SubConfig, SubAdvanced }

pub struct MainMenu {
    pub selected: usize,
    pub mode: MenuMode,
    pub language: Language,
    pub server_running: bool,
    pub mc_minder_running: bool,
    pub watchdog_running: bool,
    pub mc_status: Option<McStatusSnapshot>,
    pub discovered_servers: Vec<String>,
    pub session_name: String,
    pub tps_history: VecDeque<f64>,
}

impl MainMenu {
    pub fn new(language: Language) -> Self {
        Self { selected: 0, mode: MenuMode::Main, language, server_running: false, mc_minder_running: false, watchdog_running: false, mc_status: None, discovered_servers: Vec::new(), session_name: String::new(), tps_history: VecDeque::new() }
    }

    fn main_items(&self) -> Vec<&'static str> {
        match self.language {
            Language::Chinese => vec!["服务器控制","监控与日志","配置与管理","高级工具","语言切换","退出"],
            Language::English => vec!["Server Control","Monitoring","Configuration","Advanced Tools","Language","Exit"],
        }
    }
    fn sub_items(&self) -> Vec<&'static str> {
        match (self.mode, self.language) {
            (MenuMode::SubServer, Language::Chinese) => vec!["启动(后台)","启动(前台)","停止服务器","重启服务器","返回"],
            (MenuMode::SubServer, _) => vec!["Start(Bg)","Start(Fg)","Stop","Restart","Back"],
            (MenuMode::SubMonitor, Language::Chinese) => vec!["服务器状态","控制台","服务器日志","MC-Minder日志","备份列表","已安装Mod","返回"],
            (MenuMode::SubMonitor, _) => vec!["Status","Console","Server Log","MC-Minder Log","Backups","Mods","Back"],
            (MenuMode::SubConfig, Language::Chinese) => vec!["初始化配置","更新MC-Minder","Java管理","编辑配置","返回"],
            (MenuMode::SubConfig, _) => vec!["Init Config","Update","Java","Edit Config","Back"],
            (MenuMode::SubAdvanced, Language::Chinese) => vec!["新建服务器","Mod下载","备份世界","快捷指令","返回"],
            (MenuMode::SubAdvanced, _) => vec!["New Server","Mods","Backup World","Quick Cmds","Back"],
            _ => vec![],
        }
    }
    fn sub_mode_title(&self) -> &'static str {
        match self.mode { MenuMode::SubServer => "Server Control", MenuMode::SubMonitor => "Monitoring", MenuMode::SubConfig => "Configuration", MenuMode::SubAdvanced => "Advanced Tools", MenuMode::Main => "" }
    }
    fn status_text(&self) -> String {
        let mut s = String::new();
        if let Some(ref mc) = self.mc_status {
            if mc.online {
                s.push_str(&format!("MC 服务器: 在线\n  版本: {}\n  玩家: {}/{}\n  延迟: {}ms\n\n", mc.version, mc.players_online, mc.players_max, mc.latency_ms));
            } else {
                s.push_str(&format!("MC 服务器: 离线\n  原因: {}\n\n", mc.error.as_deref().unwrap_or("未知")));
            }
        } else if matches!(self.language, Language::Chinese) { s.push_str("MC 服务器: 等待数据...\n\n"); }
        else { s.push_str("MC Server: Waiting...\n\n"); }
        s.push_str(&format!("进程状态:\n  tmux: {} ({})\n  MC-Minder: {}\n  看门狗: {}\n", self.session_name, if self.server_running {"ON"} else {"OFF"}, if self.mc_minder_running {"ON"} else {"OFF"}, if self.watchdog_running {"ON"} else {"OFF"}));
        s
    }
}

impl Component for MainMenu {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        let n = match self.mode { MenuMode::Main => 6, _ => self.sub_items().len() };
        match key.code {
            KeyCode::Char('q') if self.mode == MenuMode::Main => Action::Quit,
            KeyCode::Esc | KeyCode::Char('q') => if self.mode == MenuMode::Main { Action::Noop } else { self.mode = MenuMode::Main; self.selected = 0; Action::Noop },
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => { self.selected = (self.selected + 1) % n; Action::Noop }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => { self.selected = if self.selected == 0 { n - 1 } else { self.selected - 1 }; Action::Noop }
            KeyCode::Enter => match self.mode {
                MenuMode::Main => match self.selected {
                    0 => { self.mode = MenuMode::SubServer; self.selected = 0; Action::Noop }
                    1 => { self.mode = MenuMode::SubMonitor; self.selected = 0; Action::Noop }
                    2 => { self.mode = MenuMode::SubConfig; self.selected = 0; Action::Noop }
                    3 => { self.mode = MenuMode::SubAdvanced; self.selected = 0; Action::Noop }
                    4 => Action::Navigate(AppState::LanguageSelect),
                    5 => Action::Navigate(AppState::ConfirmDialog(ConfirmAction::Exit)),
                    _ => Action::Noop,
                },
                MenuMode::SubServer => match self.selected {
                    0 => Action::StartServerBackground,
                    1 => Action::StartServerForeground,
                    2 => Action::Navigate(AppState::ConfirmDialog(ConfirmAction::StopServer)),
                    3 => Action::Navigate(AppState::ConfirmDialog(ConfirmAction::RestartServer)),
                    _ => { self.mode = MenuMode::Main; self.selected = 0; Action::Noop }
                },
                MenuMode::SubMonitor => match self.selected {
                    0 => { self.mode = MenuMode::Main; Action::Navigate(AppState::StatusView) }
                    1 => { self.mode = MenuMode::Main; Action::Navigate(AppState::Console) }
                    2 => { self.mode = MenuMode::Main; Action::Navigate(AppState::LogViewer(LogType::Server)) }
                    3 => { self.mode = MenuMode::Main; Action::Navigate(AppState::LogViewer(LogType::McMinder)) }
                    4 => { self.mode = MenuMode::Main; Action::Navigate(AppState::BackupList) }
                    5 => { self.mode = MenuMode::Main; Action::Navigate(AppState::ModList) }
                    _ => { self.mode = MenuMode::Main; self.selected = 0; Action::Noop }
                },
                MenuMode::SubConfig => match self.selected {
                    0 => Action::Navigate(AppState::ConfigWizard),
                    1 => Action::Navigate(AppState::ConfirmDialog(ConfirmAction::UpdateMcminder)),
                    2 => Action::Navigate(AppState::JavaMenu),
                    3 => Action::Navigate(AppState::ServerConfigEdit),
                    _ => { self.mode = MenuMode::Main; self.selected = 0; Action::Noop }
                },
                MenuMode::SubAdvanced => match self.selected {
                    0 => Action::Navigate(AppState::NewServerWizard),
                    1 => Action::Navigate(AppState::ModBrowser),
                    2 => Action::CreateBackup,
                    3 => Action::Navigate(AppState::QuickCommands),
                    _ => { self.mode = MenuMode::Main; self.selected = 0; Action::Noop }
                },
            },
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match self.mode {
            MenuMode::Main => {
                let chunks = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Length(45), Constraint::Min(0)]).split(area);
                let items = self.main_items();
                let list_items: Vec<ListItem> = items.iter().enumerate().map(|(i, text)| {
                    let color = match i { 0..=3 => Color::Green, 4..=7 => Color::Cyan, 8..=13 => Color::Yellow, _ => Color::Magenta };
                    ListItem::new(Span::styled(*text, Style::default().fg(color)))
                }).collect();
                let mut state = ListState::default(); state.select(Some(self.selected));
                let list = List::new(list_items).block(Block::default().borders(Borders::ALL)
                    .title(match self.language { Language::Chinese => "MC-Minder 管理菜单", Language::English => "MC-Minder Management Menu" }))
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
                frame.render_stateful_widget(list, chunks[0], &mut state);
                let status = Paragraph::new(self.status_text()).block(Block::default().borders(Borders::ALL)
                    .title(match self.language { Language::Chinese => "状态", Language::English => "Status" }));
                frame.render_widget(status, chunks[1]);
                let help = match self.language { Language::Chinese => "上下键: 导航 | Enter: 确认 | 1-9: 快速选择 | q: 退出", Language::English => "Up/Down: Navigate | Enter: Select | 1-9: Quick | q: Quit" };
                crate::tui::widgets::help_bar::render(frame, area, help);
            }
            _ => {
                let items = self.sub_items();
                let li: Vec<ListItem> = items.iter().map(|t| ListItem::new(Span::raw(*t))).collect();
                let mut s = ListState::default(); s.select(Some(self.selected));
                let l = List::new(li).block(Block::default().title(self.sub_mode_title()).borders(Borders::ALL))
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
                let popup = centered_rect(40, 18, area);
                frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), popup);
                frame.render_stateful_widget(l, popup, &mut s);
            }
        }
    }
}

// Java Menu — navigation hub for Java version management.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::tui::component::Component;
use crate::tui::state::{AppState, Language};
use crate::tui::action::Action;
use crate::tui::services::java_manager;

pub struct JavaMenu {
    pub selected: usize,
    pub language: Language,
    pub versions: Vec<(String, String)>,
}

impl JavaMenu {
    pub fn new(language: Language, config: Option<&crate::config::Config>) -> Self {
        let versions = java_manager::detect_java_versions(config);
        Self { selected: 0, language, versions }
    }
    fn items(&self) -> Vec<&'static str> {
        match self.language {
            Language::Chinese => vec!["1. 切换 Java 版本", "2. 安装新 Java 版本", "3. 查看所有已安装版本", "4. 返回主菜单"],
            Language::English => vec!["1. Switch Java Version", "2. Install New Java Version", "3. View All Installed Versions", "4. Back to Main Menu"],
        }
    }
}

impl Component for JavaMenu {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        let n = 4;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::Navigate(AppState::MainMenu),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') if self.selected + 1 < n => { self.selected += 1; Action::Noop }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => { self.selected = self.selected.saturating_sub(1); Action::Noop }
            KeyCode::Enter => match self.selected {
                0 => Action::Navigate(AppState::JavaSwitch(self.versions.clone())),
                1 => Action::Navigate(AppState::JavaInstall),
                2 => { self.selected = 0; Action::Navigate(AppState::MainMenu) }
                3 => Action::Navigate(AppState::MainMenu),
                _ => Action::Noop,
            },
            _ => Action::Noop,
        }
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items = self.items();
        let list_items: Vec<ListItem> = items.iter().map(|i| ListItem::new(Span::raw(*i))).collect();
        let mut state = ListState::default(); state.select(Some(self.selected));
        let list = List::new(list_items).block(Block::default().borders(Borders::ALL)
            .title(match self.language { Language::Chinese => "Java 版本管理", Language::English => "Java Version Management" }))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }
}

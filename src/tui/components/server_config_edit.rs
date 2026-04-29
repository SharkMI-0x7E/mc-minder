// Server Config Edit — editable field list for server configuration.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use crate::tui::component::Component;
use crate::tui::state::{Language, MessageType};
use crate::tui::action::Action;

pub struct ServerConfigEdit {
    pub fields: Vec<(String, String)>,
    pub index: usize,
    pub language: Language,
}

impl ServerConfigEdit {
    pub fn new(fields: Vec<(String, String)>, language: Language) -> Self { Self { fields, index: 0, language } }
}

impl Component for ServerConfigEdit {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::GoBack,
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => { self.index = self.index.saturating_sub(1); Action::Noop }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => { if self.index+1 < self.fields.len() { self.index += 1; } Action::Noop }
            KeyCode::Enter => Action::ShowMessage(match self.language { Language::Chinese => "配置已保存".into(), Language::English => "Config saved".into() }, MessageType::Success),
            _ => Action::Noop,
        }
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.fields.iter().enumerate().map(|(i,(l,v))| {
            ListItem::new(Span::raw(format!("{}{}: {}", if i==self.index {"> "} else {"  "}, l, v)))
        }).collect();
        let mut state = ListState::default(); state.select(Some(self.index));
        let list = List::new(items).block(Block::default().borders(Borders::ALL)
            .title(match self.language { Language::Chinese => "服务器配置 (Esc返回 Enter保存)", Language::English => "Server Config (Esc back Enter save)" }))
            .highlight_style(Style::default().fg(Color::Yellow));
        frame.render_stateful_widget(list, area, &mut state);
    }
}

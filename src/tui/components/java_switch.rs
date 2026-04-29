// Java Switch — interactive version selector from detected Java installations.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::tui::component::Component;
use crate::tui::state::Language;
use crate::tui::action::Action;
use crate::tui::widgets::centered_rect::centered_rect;

pub struct JavaSwitch {
    pub selected: usize,
    pub versions: Vec<(String, String)>,
    pub language: Language,
}

impl JavaSwitch {
    pub fn new(versions: Vec<(String, String)>, language: Language) -> Self {
        Self { selected: 0, versions, language }
    }
}

impl Component for JavaSwitch {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        let max = self.versions.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::GoBack,
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => { self.selected = self.selected.saturating_sub(1); Action::Noop }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => { if self.selected < max { self.selected += 1; } Action::Noop }
            KeyCode::Enter => {
                if let Some((path, ver)) = self.versions.get(self.selected) {
                    let jdk_path = if path.contains("system default") { String::new() } else if path.ends_with("/java") { path.clone() } else { format!("{}/bin/java", path) };
                    Action::SwitchJava { path: jdk_path, version: ver.clone() }
                } else { Action::Noop }
            }
            _ => Action::Noop,
        }
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.versions.iter().map(|(path, ver)| {
            let label = if path.contains("system default") { format!("(system) {}  [{}]", ver, path) } else { format!("{}  [{}]", ver, path) };
            ListItem::new(Span::raw(label))
        }).collect();
        let mut state = ListState::default(); state.select(Some(self.selected));
        let list = List::new(items).block(Block::default().borders(Borders::ALL)
            .title(match self.language { Language::Chinese => "选择 Java 版本 (Enter确认 Esc返回)", Language::English => "Select Java Version (Enter to confirm Esc to cancel)" }))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
        let popup = centered_rect(60, 30, area);
        frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), popup);
        frame.render_stateful_widget(list, popup, &mut state);
    }
}

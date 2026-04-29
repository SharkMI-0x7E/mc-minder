// Language selection component — first-launch language choice and manual switching.
// Extracted from app.rs draw_language_select / on_key_language_select.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::tui::component::Component;
use crate::tui::state::{AppState, Language};
use crate::tui::widgets::centered_rect::centered_rect;
use crate::tui::action::Action;

pub struct LanguageSelect {
    pub selected: Language,
}

impl LanguageSelect {
    pub fn new(default: Language) -> Self {
        Self { selected: default }
    }
}

impl Component for LanguageSelect {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Navigate(AppState::MainMenu),
            KeyCode::Char('1') | KeyCode::Char('c') => Action::SetLanguage(Language::Chinese),
            KeyCode::Char('2') | KeyCode::Char('e') => Action::SetLanguage(Language::English),
            KeyCode::Up | KeyCode::Down => { self.selected = self.selected.toggle(); Action::Noop }
            KeyCode::Enter => Action::SetLanguage(self.selected),
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items = vec![
            ListItem::new(Span::raw("1. 中文 (Chinese)")),
            ListItem::new(Span::raw("2. English (英文)")),
        ];
        let mut state = ListState::default();
        state.select(Some(match self.selected { Language::Chinese => 0, Language::English => 1 }));
        let list = List::new(items)
            .block(Block::default().title("语言设置 / Language Settings").borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        let popup = centered_rect(50, 30, area);
        frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), popup);
        frame.render_stateful_widget(list, popup, &mut state);
    }
}

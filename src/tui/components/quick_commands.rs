// Quick Commands component — fixed-option RCON command menu.
// Extracted from app.rs draw_quick_commands / on_key_quick_commands.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::Frame;

use crate::tui::component::Component;
use crate::tui::state::{AppState, Language, MessageType};
use crate::tui::action::Action;
use crate::tui::widgets::centered_rect::centered_rect;

pub struct QuickCommands {
    pub language: Language,
}

impl QuickCommands {
    pub fn new(language: Language) -> Self { Self { language } }
}

impl Component for QuickCommands {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::GoBack,
            KeyCode::Char('1') => Action::ShowMessage("Sent: difficulty peaceful".into(), MessageType::Success),
            KeyCode::Char('2') => Action::ShowMessage("Sent: time set day".into(), MessageType::Success),
            KeyCode::Char('3') => Action::ShowMessage("Sent: weather clear".into(), MessageType::Success),
            KeyCode::Char('4') => Action::ShowMessage("Sent: gamerule keepInventory true".into(), MessageType::Success),
            KeyCode::Char('5') => Action::Navigate(AppState::StatusView),
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = match self.language { Language::Chinese => "快捷命令", Language::English => "Quick Commands" };
        let items: Vec<ListItem> = if matches!(self.language, Language::English) {
            vec![
                ListItem::new(Span::raw("1. Peaceful Mode  (difficulty peaceful)")),
                ListItem::new(Span::raw("2. Day Time      (time set day)")),
                ListItem::new(Span::raw("3. Clear Weather (weather clear)")),
                ListItem::new(Span::raw("4. KeepInventory (gamerule keepInventory true)")),
                ListItem::new(Span::raw("5. View Status")),
            ]
        } else {
            vec![
                ListItem::new(Span::raw("1. 和平模式     (difficulty peaceful)")),
                ListItem::new(Span::raw("2. 白天时间     (time set day)")),
                ListItem::new(Span::raw("3. 清除天气     (weather clear)")),
                ListItem::new(Span::raw("4. 保留物品     (gamerule keepInventory true)")),
                ListItem::new(Span::raw("5. 查看状态")),
            ]
        };
        let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));
        let inner = centered_rect(55, 25, area);
        frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), inner);
        frame.render_widget(list, inner);
    }
}

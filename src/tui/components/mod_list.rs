// Mod List component — displays installed mods (read-only).
// Extracted from app.rs draw_mod_list / on_key_mod_list.

use std::path::Path;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::style::{Color, Style};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::component::Component;
use crate::tui::state::Language;
use crate::tui::action::Action;
use crate::tui::widgets::centered_rect::centered_rect;
use crate::core_download::scan_installed_mods;

pub struct ModList {
    pub language: Language,
}

impl ModList {
    pub fn new(language: Language) -> Self { Self { language } }
}

impl Component for ModList {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::GoBack,
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let mods_dir = Path::new("mods");
        let mods = scan_installed_mods(mods_dir);
        let items: Vec<ListItem> = if mods.is_empty() {
            vec![ListItem::new(Span::raw("No mods installed (create mods/ folder and add .jar files)"))]
        } else {
            mods.iter().map(|m| ListItem::new(Span::raw(m.clone()))).collect()
        };
        let list = List::new(items).block(Block::default().title("Installed Mods (Esc back)").borders(Borders::ALL));
        let inner = centered_rect(55, 30, area);
        frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), inner);
        frame.render_widget(list, inner);
    }
}

// Mod Browser component — Modrinth popular mods downloader.
// Extracted from app.rs draw_mod_browser / on_key_mod_browser.

use std::path::PathBuf;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::component::Component;
use crate::tui::state::{AppState, Language};
use crate::tui::action::Action;
use crate::tui::widgets::centered_rect::centered_rect;

pub struct ModBrowser {
    pub selected: usize,
    pub language: Language,
}

impl ModBrowser {
    pub fn new(language: Language) -> Self { Self { selected: 0, language } }
}

impl Component for ModBrowser {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        let mods = crate::core_download::popular_mods();
        if mods.is_empty() { return Action::GoBack; }
        let max = mods.len().saturating_sub(1);

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::Navigate(AppState::MainMenu),
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => {
                self.selected = self.selected.saturating_sub(1); Action::Noop
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => {
                if self.selected < max { self.selected += 1; } Action::Noop
            }
            KeyCode::Enter => {
                if let Some((_slug, name, project_id)) = mods.get(self.selected) {
                    let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                    let rt = tokio::runtime::Handle::current();
                    let game_ver = "1.21.1";
                    let _ = rt.block_on(crate::core_download::get_modrinth_latest_version(project_id, game_ver))
                        .map(|file| {
                            let filename = file.filename.clone();
                            let _ = rt.block_on(crate::core_download::download_modrinth_mod(&file.url, &filename, &dir));
                        });
                }
                Action::Navigate(AppState::MainMenu)
            }
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let mods = crate::core_download::popular_mods();
        let items: Vec<ListItem> = mods.iter().map(|(_s, name, _)| ListItem::new(Span::raw(*name))).collect();
        let mut state = ListState::default();
        state.select(Some(self.selected));
        let title = match self.language {
            Language::Chinese => "热门 Mod (Enter下载 Esc返回)",
            Language::English => "Popular Mods (Enter download Esc back)",
        };
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        let inner = centered_rect(50, 25, area);
        frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), inner);
        frame.render_stateful_widget(list, inner, &mut state);
    }
}

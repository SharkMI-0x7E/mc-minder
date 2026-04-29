// Java Install — platform-specific JDK installation picker.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::tui::component::Component;
use crate::tui::state::{Language, MessageType};
use crate::tui::action::Action;
use crate::tui::widgets::centered_rect::centered_rect;
use crate::tui::services::java_manager;

pub struct JavaInstall {
    pub selected: usize,
    pub language: Language,
}

impl JavaInstall {
    pub fn new(language: Language) -> Self { Self { selected: 0, language } }
}

impl Component for JavaInstall {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        let options = java_manager::java_install_options();
        let max = options.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::GoBack,
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => { self.selected = self.selected.saturating_sub(1); Action::Noop }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => { if self.selected < max { self.selected += 1; } Action::Noop }
            KeyCode::Enter => {
                if let Some((label, cmd, _)) = options.get(self.selected) {
                    if cmd.is_empty() {
                        Action::ShowMessage(format!("Please install {} manually", label), MessageType::Warning)
                    } else {
                        let result = std::process::Command::new("sh").args(["-c", cmd]).output();
                        match result {
                            Ok(out) if out.status.success() => Action::ShowMessage(format!("{} installed!", label), MessageType::Success),
                            Ok(out) => Action::ShowMessage(format!("Failed: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("?")), MessageType::Warning),
                            Err(e) => Action::ShowMessage(format!("Error: {}", e), MessageType::Warning),
                        }
                    }
                } else { Action::Noop }
            }
            _ => Action::Noop,
        }
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let options = java_manager::java_install_options();
        let items: Vec<ListItem> = options.iter().map(|(label, cmd, _)| {
            let display = if cmd.is_empty() { format!("{} (manual)", label) } else { label.clone() };
            ListItem::new(Span::raw(display))
        }).collect();
        let mut state = ListState::default(); state.select(Some(self.selected));
        let list = List::new(items).block(Block::default().borders(Borders::ALL)
            .title(match self.language { Language::Chinese => "安装 Java 版本", Language::English => "Install Java Version" }))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
        let popup = centered_rect(55, 25, area);
        frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), popup);
        frame.render_stateful_widget(list, popup, &mut state);
    }
}

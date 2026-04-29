// Real-time Console viewer — tmux capture-pane with auto-refresh.
use std::time::Instant;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::{Color, Style};
use crate::tui::component::Component;
use crate::tui::state::Language;
use crate::tui::action::Action;
use crate::tui::widgets::help_bar;

pub struct ConsoleView {
    pub content: String, pub scroll: usize, pub auto_refresh: bool,
    pub last_refresh: Instant, pub session_name: String, pub language: Language,
}

impl ConsoleView {
    pub fn new(language: Language) -> Self {
        Self { content: String::new(), scroll: 0, auto_refresh: true, last_refresh: Instant::now(), session_name: String::new(), language }
    }
    pub fn capture_output(&mut self) {
        let output = std::process::Command::new("tmux").args(["capture-pane", "-p", "-t", &self.session_name]).output();
        if let Ok(out) = output {
            self.content = format!("[{}] Console output:\n{}", chrono::Local::now().format("%H:%M:%S"), String::from_utf8_lossy(&out.stdout));
            self.last_refresh = Instant::now();
        } else { self.content = match self.language { Language::Chinese => "无法捕获控制台输出".into(), Language::English => "Cannot capture console output".into() }; }
    }
}

impl Component for ConsoleView {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::GoBack,
            KeyCode::Up | KeyCode::Char('k') => { if self.scroll > 0 { self.scroll -= 1; } Action::Noop }
            KeyCode::Down | KeyCode::Char('j') => { self.scroll += 1; Action::Noop }
            KeyCode::PageUp => { self.scroll = self.scroll.saturating_sub(10); Action::Noop }
            KeyCode::PageDown => { self.scroll += 10; Action::Noop }
            KeyCode::Char('r') => { self.capture_output(); Action::Noop }
            KeyCode::Char('a') => { self.auto_refresh = !self.auto_refresh; Action::Noop }
            _ => Action::Noop,
        }
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let lines: Vec<&str> = self.content.lines().collect();
        let total = lines.len();
        let start = self.scroll.min(total.saturating_sub(1));
        let visible: String = lines.iter().skip(start).take(30).copied().collect::<Vec<_>>().join("\n");
        let auto = if self.auto_refresh { "ON" } else { "OFF" };
        frame.render_widget(Paragraph::new(visible).block(Block::default().borders(Borders::ALL)
            .title(format!("实时控制台 (Auto: {}) | {}/{}", auto, start+1, total.max(1)))), area);
        help_bar::render(frame, area, match self.language {
            Language::Chinese => "上下键: 滚动 | r: 刷新 | a: 自动刷新 | Esc: 返回",
            Language::English => "Up/Down: Scroll | r: Refresh | a: Auto | Esc: Back",
        });
    }
}

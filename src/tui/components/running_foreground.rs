// Running Foreground view — displays console output from foreground server process.
// The process management stays in App; this component only renders output lines.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Line};

use crate::tui::component::Component;
use crate::tui::state::Language;
use crate::tui::action::Action;

pub struct RunningForeground {
    pub console_lines: Vec<String>,
    pub scroll: usize,
    pub language: Language,
    pub is_running: bool,
}

impl RunningForeground {
    pub fn new(language: Language) -> Self {
        Self { console_lines: Vec::new(), scroll: 0, language, is_running: false }
    }
}

impl Component for RunningForeground {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::GoBack,
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let title = if matches!(self.language, Language::Chinese) {
            "前台服务器运行中 (q:返回菜单)"
        } else {
            "Foreground Server Running (q:Back)"
        };
        frame.render_widget(Paragraph::new(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))), chunks[0]);

        let max_lines = chunks[1].height as usize;
        let total = self.console_lines.len();
        if total > max_lines { self.scroll = total - max_lines; } else { self.scroll = 0; }

        let visible: Vec<Line> = self.console_lines.iter().skip(self.scroll).take(max_lines)
            .map(|line| Line::from(Span::raw(line.clone()))).collect();

        let console_title = if matches!(self.language, Language::Chinese) { "服务器输出" } else { "Server Output" };
        frame.render_widget(Paragraph::new(visible).block(Block::default().borders(Borders::ALL).title(console_title)), chunks[1]);

        let status = if self.is_running {
            if matches!(self.language, Language::Chinese) { "状态: 运行中" } else { "Status: Running" }
        } else {
            if matches!(self.language, Language::Chinese) { "状态: 已停止" } else { "Status: Stopped" }
        };
        frame.render_widget(Paragraph::new(Span::styled(status, Style::default().fg(Color::Green))), chunks[2]);
    }
}

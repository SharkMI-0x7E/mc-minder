// Log Viewer component — displays server or mc-minder logs with scrolling.
// Extracted from app.rs draw_log_viewer / on_key_log_viewer.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};

use crate::tui::component::Component;
use crate::tui::state::{LogType, Language};
use crate::tui::action::Action;
use crate::tui::widgets::help_bar;

pub struct LogViewer {
    pub log_type: LogType,
    pub content: String,
    pub scroll: usize,
    pub language: Language,
}

impl LogViewer {
    pub fn new(log_type: LogType, language: Language) -> Self {
        Self { log_type, content: String::new(), scroll: 0, language }
    }

    pub fn load(&mut self) {
        let path = match self.log_type {
            LogType::Server => "logs/latest.log",
            LogType::McMinder => "logs/mc-minder.log",
        };
        self.content = std::fs::read_to_string(path).unwrap_or_else(|_| {
            match self.language {
                Language::Chinese => "无法读取日志文件".to_string(),
                Language::English => "Cannot read log file".to_string(),
            }
        });
        self.scroll = 0;
    }
}

impl Component for LogViewer {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::GoBack,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll > 0 { self.scroll -= 1; }
                Action::Noop
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll += 1;
                Action::Noop
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
                Action::Noop
            }
            KeyCode::PageDown => {
                self.scroll += 10;
                Action::Noop
            }
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = match self.log_type {
            LogType::Server => match self.language {
                Language::Chinese => "服务器日志",
                Language::English => "Server Log",
            },
            LogType::McMinder => match self.language {
                Language::Chinese => "MC-Minder 日志",
                Language::English => "MC-Minder Log",
            },
        };

        let lines: Vec<&str> = self.content.lines().collect();
        let total_lines = lines.len();
        let start = self.scroll.min(total_lines.saturating_sub(1));
        let visible: String = lines.iter().skip(start).take(30)
            .map(|s| *s)
            .collect::<Vec<_>>()
            .join("\n");

        let para = Paragraph::new(visible)
            .block(Block::default()
                .title(format!("{} ({}-{} / {})", title, start + 1, (start + 30).min(total_lines), total_lines))
                .borders(Borders::ALL))
            .scroll((0, 0));
        frame.render_widget(para, area);

        let help = match self.language {
            Language::Chinese => "上下键: 滚动 | PageUp/PageDown: 翻页 | Esc: 返回",
            Language::English => "Up/Down: Scroll | PageUp/PageDown: Page | Esc: Back",
        };
        help_bar::render(frame, area, help);
    }
}

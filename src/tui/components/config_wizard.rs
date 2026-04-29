// Configuration Wizard component — interactive multi-field form.
// Extracted from app.rs draw_config_wizard / on_key_config_wizard.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::tui::component::Component;
use crate::tui::state::Language;
use crate::tui::action::Action;
use crate::tui::widgets::centered_rect::centered_rect;
use crate::config::Config;

#[derive(Clone)]
pub struct WizardField {
    pub label: &'static str,
    pub value: String,
}

pub struct ConfigWizard {
    pub fields: Vec<WizardField>,
    pub index: usize,
    pub language: Language,
}

impl ConfigWizard {
    pub fn new(language: Language, config: &Option<Config>) -> Self {
        let mut wizard = Self { fields: Vec::new(), index: 0, language };
        wizard.init_fields(config);
        wizard
    }

    pub fn init_fields(&mut self, config: &Option<Config>) {
        let c = config.as_ref();
        let jdk = c.and_then(|cc| cc.jvm.jdk_path.clone()).unwrap_or_default();
        self.fields = vec![
            WizardField { label: "Server JAR", value: c.map(|cc| cc.server.jar.clone()).unwrap_or_default() },
            WizardField { label: "Min Memory", value: c.map(|cc| cc.server.min_mem.clone()).unwrap_or_default() },
            WizardField { label: "Max Memory", value: c.map(|cc| cc.server.max_mem.clone()).unwrap_or_default() },
            WizardField { label: "Session Name", value: c.map(|cc| cc.server.session_name.clone()).unwrap_or_default() },
            WizardField { label: "RCON Port", value: c.map(|cc| cc.rcon.port.to_string()).unwrap_or_default() },
            WizardField { label: "RCON Password", value: c.map(|cc| cc.rcon.password.clone()).unwrap_or_default() },
            WizardField { label: "JDK Path", value: jdk },
        ];
    }
}

impl Component for ConfigWizard {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::GoBack,
            KeyCode::Tab => {
                if !self.fields.is_empty() {
                    self.index = (self.index + 1) % self.fields.len();
                }
                Action::Noop
            }
            KeyCode::BackTab => {
                if !self.fields.is_empty() {
                    if self.index == 0 { self.index = self.fields.len() - 1; }
                    else { self.index -= 1; }
                }
                Action::Noop
            }
            KeyCode::Enter => Action::GoBack,
            KeyCode::Char(ch) => {
                if let Some(field) = self.fields.get_mut(self.index) {
                    field.value.push(ch);
                }
                Action::Noop
            }
            KeyCode::Backspace => {
                if let Some(field) = self.fields.get_mut(self.index) {
                    field.value.pop();
                }
                Action::Noop
            }
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = match self.language {
            Language::Chinese => "配置向导",
            Language::English => "Configuration Wizard",
        };
        let items: Vec<ListItem> = self.fields.iter().enumerate().map(|(idx, fw)| {
            let mut line = String::new();
            if idx == self.index { line.push_str("> "); } else { line.push_str("  "); }
            line.push_str(&format!("{}: {}", fw.label, fw.value));
            ListItem::new(Span::raw(line))
        }).collect();
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL));
        let popup = centered_rect(70, 60, area);
        frame.render_widget(list, popup);

        let hint = match self.language {
            Language::Chinese => "Tab/Shift+Tab 切换字段, Enter 保存, Esc 取消",
            Language::English => "Tab/Shift+Tab to switch fields, Enter to save, Esc to cancel",
        };
        use ratatui::layout::{Constraint, Direction, Layout};
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        frame.render_widget(ratatui::widgets::Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)), chunks[1]);
    }
}

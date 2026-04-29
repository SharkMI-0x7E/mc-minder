// Confirmation dialog component — Y/N modal for destructive actions.
// Extracted from app.rs draw_confirm_dialog / on_key_confirm_dialog.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::{Color, Style};

use crate::tui::component::Component;
use crate::tui::state::{ConfirmAction, Language};
use crate::tui::widgets::centered_rect::centered_rect;
use crate::tui::action::Action;

pub struct ConfirmDialog {
    pub action: ConfirmAction,
    pub language: Language,
}

impl ConfirmDialog {
    pub fn new(action: ConfirmAction, language: Language) -> Self {
        Self { action, language }
    }
}

impl Component for ConfirmDialog {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => match &self.action {
                ConfirmAction::StopServer => Action::StopServer,
                ConfirmAction::RestartServer => Action::RestartServer,
                ConfirmAction::UpdateMcminder => Action::CheckUpdate,
                ConfirmAction::Exit => Action::Quit,
                ConfirmAction::Modal { .. } => Action::GoBack,
            },
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Action::GoBack,
            _ => Action::Noop,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let (title, msg) = match &self.action {
            ConfirmAction::StopServer => (
                match self.language { Language::Chinese => "确认停止", Language::English => "Confirm Stop" },
                match self.language { Language::Chinese => "确定要停止服务器吗？\n\n按 Y 确认，按 N 取消", Language::English => "Are you sure?\n\nPress Y to confirm, N to cancel" },
            ),
            ConfirmAction::RestartServer => (
                match self.language { Language::Chinese => "确认重启", Language::English => "Confirm Restart" },
                match self.language { Language::Chinese => "确定要重启服务器吗？\n\n按 Y 确认，按 N 取消", Language::English => "Are you sure?\n\nPress Y to confirm, N to cancel" },
            ),
            ConfirmAction::UpdateMcminder => (
                match self.language { Language::Chinese => "确认更新", Language::English => "Confirm Update" },
                match self.language { Language::Chinese => "确定要更新 MC-Minder 吗？\n\n按 Y 确认，按 N 取消", Language::English => "Are you sure?\n\nPress Y to confirm, N to cancel" },
            ),
            ConfirmAction::Exit => (
                match self.language { Language::Chinese => "确认退出", Language::English => "Confirm Exit" },
                match self.language { Language::Chinese => "确定要退出吗？\n\n按 Y 确认，按 N 取消", Language::English => "Are you sure?\n\nPress Y to confirm, N to cancel" },
            ),
            ConfirmAction::Modal { title_cn, title_en, message_cn, message_en } => (
                if matches!(self.language, Language::Chinese) { *title_cn } else { *title_en },
                if matches!(self.language, Language::Chinese) { *message_cn } else { *message_en },
            ),
        };
        let para = Paragraph::new(msg)
            .block(Block::default().title(title).borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center);
        let inner = centered_rect(50, 30, area);
        frame.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), inner);
        frame.render_widget(para, inner);

        let help = match self.language {
            Language::Chinese => "Y: 确认 | N: 取消 | Esc: 返回",
            Language::English => "Y: Confirm | N: Cancel | Esc: Back",
        };
        let help_para = Paragraph::new(help).style(Style::default().fg(Color::DarkGray));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        frame.render_widget(help_para, chunks[1]);
    }
}

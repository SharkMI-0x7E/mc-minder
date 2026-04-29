// Update view — mc-minder self-update progress display.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use crate::tui::component::Component;
use crate::tui::state::{UpdateState, Language};
use crate::tui::action::Action;
use crate::tui::widgets::help_bar;

pub struct UpdateView {
    pub state: Option<UpdateState>,
    pub language: Language,
}

impl UpdateView {
    pub fn new(language: Language) -> Self { Self { state: None, language } }
}

impl Component for UpdateView {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::GoBack,
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => match &self.state {
                Some(UpdateState::UpdateAvailable { download_url, latest, .. }) => Action::StartUpdate { url: download_url.clone(), version: latest.clone() },
                Some(UpdateState::UpToDate) | Some(UpdateState::Failed(_)) => Action::GoBack,
                Some(UpdateState::Done { .. }) => Action::Quit,
                _ => Action::Noop,
            },
            KeyCode::Char('n') | KeyCode::Char('N') => if matches!(self.state, Some(UpdateState::UpdateAvailable{..})) { Action::GoBack } else { Action::Noop },
            _ => Action::Noop,
        }
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let cn = matches!(self.language, Language::Chinese);
        let content = match &self.state {
            Some(UpdateState::UpdateAvailable{current,latest,..}) => Paragraph::new(format!("{}\n\nCurrent: {}\nLatest: {}\n\n{}",
                if cn {"发现新版本!"} else {"New version available!"}, current, latest,
                if cn {"按 Y 更新, N 取消"} else {"Y to update, N to cancel"})),
            Some(UpdateState::UpToDate) => Paragraph::new(if cn {"已是最新版本!"} else {"Up to date!"}),
            Some(UpdateState::Downloading{downloaded,total}) => {
                let d = *downloaded as f64;
                let t = match total { Some(v) => *v as f64, None => 1.0 };
                let pct = (d / t * 100.0).min(100.0);
                Paragraph::new(format!("Downloading...\n{:.0}%", pct))
            },
            Some(UpdateState::Installing) => Paragraph::new(if cn {"正在安装..."} else {"Installing..."}),
            Some(UpdateState::Done{new_version}) => Paragraph::new(format!("{} v{}!", if cn {"更新完成"} else {"Updated"}, new_version)),
            Some(UpdateState::Failed(e)) => Paragraph::new(format!("{}: {}", if cn {"失败"} else {"Failed"}, e)),
            None => Paragraph::new(if cn {"初始化中..."} else {"Initializing..."}),
        };
        frame.render_widget(content.block(Block::default().borders(Borders::ALL).title(if cn {"更新"} else {"Update"})).alignment(Alignment::Center), area);
        help_bar::render(frame, area, if cn {"Y:确认 N:取消 Esc:返回"} else {"Y:Confirm N:Cancel Esc:Back"});
    }
}

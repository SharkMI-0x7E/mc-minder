// Server Status view — process status + MC ping + TPS chart.
use std::collections::VecDeque;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use crate::tui::component::Component;
use crate::tui::state::Language;
use crate::tui::action::Action;
use crate::api::McStatusSnapshot;

pub struct StatusView {
    pub server_running: bool,
    pub mc_minder_running: bool,
    pub watchdog_running: bool,
    pub mc_status: Option<McStatusSnapshot>,
    pub tps_history: VecDeque<f64>,
    pub session_name: String,
    pub language: Language,
    pub discovered_servers: Vec<String>,
}

impl StatusView {
    pub fn new(language: Language) -> Self {
        Self { server_running: false, mc_minder_running: false, watchdog_running: false, mc_status: None, tps_history: VecDeque::new(), session_name: String::new(), language, discovered_servers: Vec::new() }
    }
    fn tps_chart_line(&self, threshold: f64, label: &str) -> String {
        let prefix = if label.is_empty() { "   ".to_string() } else { format!("{:>2} ", label) };
        let mut line = prefix;
        for &tps in &self.tps_history {
            line.push(if tps >= threshold { if tps >= 18.0 { '█' } else if tps >= 10.0 { '▓' } else { '░' } } else { ' ' });
        }
        line
    }
}

impl Component for StatusView {
    fn handle_events(&mut self, key: KeyEvent) -> Action {
        match key.code { KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => Action::GoBack, _ => Action::Noop }
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let has_tps = !self.tps_history.is_empty();
        let chunks = Layout::default().direction(Direction::Vertical).constraints([
            Constraint::Length(2), Constraint::Length(if has_tps { 3 } else { 2 }),
            Constraint::Length(if has_tps { 6 } else { 0 }), Constraint::Min(1),
        ]).split(area);
        let proc_text = format!("Session: {} | MC-Minder: {} | Watchdog: {}",
            self.session_name, if self.mc_minder_running { "ON" } else { "OFF" }, if self.watchdog_running { "ON" } else { "OFF" });
        frame.render_widget(Paragraph::new(proc_text).block(Block::default().borders(Borders::ALL).title("Process").border_style(Style::default().fg(Color::Gray))), chunks[0]);
        let (mc_title, mc_text, border_color) = if let Some(ref s) = self.mc_status {
            if s.online {
                let mut txt = format!("Version: {} | Players: {}/{} | Latency: {}ms", s.version, s.players_online, s.players_max, s.latency_ms);
                if let Some(tps) = s.tps { txt.push_str(&format!(" | TPS: {:.1}", tps)); }
                txt.push_str(&format!("\nMOTD: {}", s.motd.lines().next().unwrap_or("")));
                ("Minecraft Server", txt, Color::Green)
            } else { ("Minecraft Server", format!("OFFLINE — {}", s.error.as_deref().unwrap_or("unknown")), Color::Red) }
        } else { ("Minecraft Server", "Waiting for data...".to_string(), Color::Yellow) };
        frame.render_widget(Paragraph::new(mc_text).block(Block::default().borders(Borders::ALL).title(mc_title).border_style(Style::default().fg(border_color))), chunks[1]);
        if has_tps {
            let chart = vec![self.tps_chart_line(20.0,"20"),self.tps_chart_line(19.5,""),self.tps_chart_line(18.0,"18"),self.tps_chart_line(15.0,""),self.tps_chart_line(10.0,"10"),self.tps_chart_line(5.0,"")].join("\n");
            frame.render_widget(Paragraph::new(chart).block(Block::default().borders(Borders::ALL).title("TPS History (last 30 readings)")), chunks[2]);
        }
    }
}

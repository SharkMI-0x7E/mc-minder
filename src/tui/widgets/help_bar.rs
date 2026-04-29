// Help bar widget — renders a single-line hint bar at the bottom of the screen.
// Extracted from app.rs to avoid duplicating the same pattern across 15+ views.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

use crate::tui::state::Language;

/// Render a help/hint bar at the bottom of the current frame area.
///
/// The bar takes the last line of the available area.
///
/// # Example
/// ```ignore
/// help_bar(f, area, "Up/Down: Navigate | Enter: Select | Esc: Back", Language::English);
/// ```
pub fn render(f: &mut Frame, area: ratatui::layout::Rect, text: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let help = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[1]);
}

/// Convenience: render a bilingual help bar, selecting the right language.
pub fn render_bilingual(f: &mut Frame, area: ratatui::layout::Rect, cn_text: &str, en_text: &str, lang: Language) {
    let text = match lang {
        Language::Chinese => cn_text,
        Language::English => en_text,
    };
    render(f, area, text);
}

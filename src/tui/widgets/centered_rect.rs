// Centered popup rectangle — creates a centered area for dialogs and popups.
// Extracted from app.rs to avoid duplication across multiple views.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Create a rectangle centered within `r`, taking `percent_x`% of width
/// and `percent_y`% of height.
///
/// Used for: confirm dialogs, message popups, mod browser, wizard screens.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

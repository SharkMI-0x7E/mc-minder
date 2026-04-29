// TUI Component trait — adapted from ratatui official template
// Reference: https://ratatui.rs/templates/component/
// Reference: https://ratatui.rs/concepts/application-patterns/component-architecture/

use anyhow::Result;
use crossterm::event::{KeyEvent, KeyEventKind, KeyCode};
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::tui::action::Action;

/// Component trait matching ratatui's official component architecture.
///
/// Each view in the TUI implements this trait, encapsulating its own
/// state, event handling, and rendering logic.
///
/// # Lifecycle
/// 1. `init()` — called when component is first mounted
/// 2. `handle_events()` — called on each key event during the event loop
/// 3. `update()` — called when an Action is dispatched (from self or other components)
/// 4. `render()` — called on each frame to draw the component
pub trait Component {
    /// Called when the component is first mounted / becomes active.
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Handle a key event, returning an Action for the App to dispatch.
    ///
    /// The `key` parameter is already normalized (Release/Repeat filtered,
    /// Windows numpad mapped). See `normalize_key()`.
    fn handle_events(&mut self, key: KeyEvent) -> Action;

    /// Handle an Action dispatched by the App (from self or other components).
    /// Returns the action back (or a transformed one) for chaining.
    fn update(&mut self, action: Action) -> Action {
        action
    }

    /// Render the component into the given area of the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

/// Normalize a raw KeyEvent for cross-platform compatibility.
///
/// This function:
/// 1. Filters out `Release` and `Repeat` events (Windows terminals send both Press and Release)
/// 2. Maps Windows numpad arrow keys (NumLock ON sends Char instead of Up/Down/Left/Right)
///
/// Returns `None` if the event should be ignored entirely.
///
/// All Components should call this at the start of their `handle_events`:
/// ```ignore
/// fn handle_events(&mut self, raw_key: KeyEvent) -> Action {
///     let key = match normalize_key(raw_key) {
///         Some(k) => k,
///         None => return Action::Noop,
///     };
///     // ... handle key ...
/// }
/// ```
pub fn normalize_key(key: KeyEvent) -> Option<KeyEvent> {
    // Filter Windows double events (Press + Release both sent)
    if key.kind == KeyEventKind::Release || key.kind == KeyEventKind::Repeat {
        return None;
    }

    // Map Windows numpad arrow keys (NumLock ON)
    let code = match key.code {
        KeyCode::Char('8') => KeyCode::Up,
        KeyCode::Char('2') => KeyCode::Down,
        KeyCode::Char('4') => KeyCode::Left,
        KeyCode::Char('6') => KeyCode::Right,
        other => other,
    };

    Some(KeyEvent { code, ..key })
}

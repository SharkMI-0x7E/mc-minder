// Global Action enum — the message type passed between Components and the App.
// Matches ratatui official template pattern: https://ratatui.rs/templates/component/

use crate::tui::state::{AppState, Language, MessageType};

/// All possible actions that a Component can request or that the App can dispatch.
///
/// Components return Actions from `handle_events()`. The App dispatches them
/// via `App::dispatch()`, executing side effects and routing back to components.
#[derive(Debug, Clone)]
pub enum Action {
    // =========================================================================
    // Navigation
    // =========================================================================
    /// Navigate to a specific app state / view.
    Navigate(AppState),
    /// Go back to the previous view (typically MainMenu).
    GoBack,
    /// Exit the application.
    Quit,

    // =========================================================================
    // Server Control
    // =========================================================================
    /// Start the Minecraft server in background (tmux session).
    StartServerBackground,
    /// Start the Minecraft server in foreground (exit TUI, exec java).
    StartServerForeground,
    /// Stop the Minecraft server gracefully.
    StopServer,
    /// Restart the Minecraft server (stop + start).
    RestartServer,
    /// Refresh and display server status.
    ShowServerStatus,

    // =========================================================================
    // Update Engine
    // =========================================================================
    /// Check for a new version of mc-minder.
    CheckUpdate,
    /// Start downloading the new version.
    StartUpdate { url: String, version: String },
    /// Cancel an in-progress update.
    CancelUpdate,

    // =========================================================================
    // Java Management
    // =========================================================================
    /// Switch to the specified Java version.
    SwitchJava { path: String, version: String },
    /// Install a new Java version.
    InstallJava(String),
    /// Re-detect installed Java versions.
    RefreshJavaVersions,

    // =========================================================================
    // Configuration
    // =========================================================================
    /// Save configuration from the wizard fields.
    SaveConfigFromWizard,

    // =========================================================================
    // New Server Wizard (P3)
    // =========================================================================
    /// Open the new server creation wizard.
    StartNewServerWizard,
    /// User selected a core type (step 0).
    SelectCoreType(usize),
    /// User selected a Minecraft version (step 1).
    SelectVersion(usize),
    /// Trigger the actual download.
    StartServerDownload,

    // =========================================================================
    // Mod Management (P3-4)
    // =========================================================================
    /// Download a mod from Modrinth.
    DownloadMod { project_id: String, name: String },
    /// Open the mod browser.
    OpenModBrowser,

    // =========================================================================
    // Backup (P5)
    // =========================================================================
    /// Create a world backup.
    CreateBackup,
    /// Show the backup list.
    ListBackups,
    /// A backup operation completed (from spawn_blocking).
    BackupComplete(Result<String, String>),

    // =========================================================================
    // Logs / Console
    // =========================================================================
    /// Load server log file content.
    LoadServerLog,
    /// Load mc-minder log file content.
    LoadMinderLog,
    /// Capture console output from tmux pane.
    CaptureConsole,

    // =========================================================================
    // Foreground Process (RunningForeground view)
    // =========================================================================
    /// A line of output from the foreground server process.
    ConsoleLine(String),
    /// The foreground process has exited.
    ProcessExited(i32),
    /// Send a command to the foreground process stdin.
    SendStdin(String),

    // =========================================================================
    // UI Notifications
    // =========================================================================
    /// Show a message popup to the user.
    ShowMessage(String, MessageType),
    /// Clear the current message popup.
    ClearMessage,

    // =========================================================================
    // Language
    // =========================================================================
    /// Set the application language.
    SetLanguage(Language),

    // =========================================================================
    // System
    // =========================================================================
    /// Periodic tick (every ~200ms) — used for timeouts, auto-refresh, etc.
    Tick,
    /// Request a re-render (rarely needed explicitly).
    Render,
    /// Enter processing/busy state (blocks all input).
    EnterBusy,
    /// Exit processing/busy state.
    ClearBusy,
    /// No operation — explicitly do nothing.
    Noop,
}

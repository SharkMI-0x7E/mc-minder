// TUI state types — extracted from app.rs for modularity.
// These are pure data types with no rendering or business logic.

// ---- App State Machine ----

/// Top-level application state. Each variant corresponds to a different view/screen.
#[derive(Clone, Debug, PartialEq)]
pub enum AppState {
    MainMenu,
    SubServer,
    SubMonitor,
    SubConfig,
    SubAdvanced,
    JavaMenu,
    JavaSwitch(Vec<(String, String)>),
    JavaInstall,
    LogViewer(LogType),
    ConfigWizard,
    LanguageSelect,
    ConfirmDialog(ConfirmAction),
    StatusView,
    Console,
    UpdateView,
    RunningForeground,
    Busy(String),
    ServerConfigEdit,
    NewServerWizard,
    ModBrowser,
    QuickCommands,
    BackupList,
    ModList,
}

// ---- Update State Machine ----

#[derive(Clone, Debug)]
pub enum UpdateState {
    UpdateAvailable { current: String, latest: String, download_url: String },
    UpToDate,
    Downloading { downloaded: u64, total: Option<u64> },
    Installing,
    Done { new_version: String },
    Failed(String),
}

// ---- Misc Enums ----

#[derive(Clone, Debug, PartialEq)]
pub enum LogType { Server, McMinder }

#[derive(Clone, Debug, PartialEq)]
pub enum ConfirmAction {
    StopServer,
    RestartServer,
    UpdateMcminder,
    Exit,
    Modal {
        title_cn: &'static str,
        title_en: &'static str,
        message_cn: &'static str,
        message_en: &'static str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Language {
    Chinese,
    English,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self { Language::Chinese => "zh", Language::English => "en" }
    }
    pub fn from_code(s: &str) -> Option<Self> {
        match s.trim() { "zh" => Some(Language::Chinese), "en" => Some(Language::English), _ => None }
    }
    pub fn toggle(&self) -> Self {
        match self { Language::Chinese => Language::English, Language::English => Language::Chinese }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageType {
    Info,
    Success,
    Warning,
    Error,
}

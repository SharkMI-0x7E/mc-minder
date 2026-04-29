// Internationalization (i18n) string table for the TUI.
// All user-visible strings are centralized here, keyed by enum.
// Supports Chinese and English. New strings only need to be added in two places.

use crate::tui::state::Language;

// ============================================================================
// String key enum — compile-time safe, no typo risk
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum I18nKey {
    // ---- Main Menu ----
    MainMenuTitle,
    MainMenuStatusTitle,
    MainMenuProcessLabel,
    MainMenuMcServerLabel,
    MainMenuDiscoveredServers,
    MainMenuHelp,

    // ---- Menu Categories ----
    MenuServerControl,
    MenuMonitoring,
    MenuConfiguration,
    MenuAdvancedTools,
    MenuLanguage,
    MenuExit,

    // ---- Sub-menus ----
    // Server Control
    SubServerStartBg,
    SubServerStartFg,
    SubServerStop,
    SubServerRestart,
    SubServerBack,
    SubServerTitle,
    // Monitoring
    SubMonitorStatus,
    SubMonitorConsole,
    SubMonitorServerLog,
    SubMonitorMinderLog,
    SubMonitorBackups,
    SubMonitorMods,
    SubMonitorBack,
    SubMonitorTitle,
    // Configuration
    SubConfigInit,
    SubConfigUpdate,
    SubConfigJava,
    SubConfigEdit,
    SubConfigBack,
    SubConfigTitle,
    // Advanced
    SubAdvancedNewServer,
    SubAdvancedMods,
    SubAdvancedBackup,
    SubAdvancedQuickCmds,
    SubAdvancedBack,
    SubAdvancedTitle,

    // ---- Confirm Dialog ----
    ConfirmTitle,
    ConfirmStopMsg,
    ConfirmRestartMsg,
    ConfirmUpdateMsg,
    ConfirmExitMsg,
    ConfirmYesNo,

    // ---- Status View ----
    StatusProcessTitle,
    StatusMcServerTitle,
    StatusTpsHistory,
    StatusOnline,
    StatusOffline,
    StatusWaitingForData,
    StatusProcessRunning,
    StatusProcessStopped,
    StatusProcessNotStarted,

    // ---- Console ----
    ConsoleTitle,
    ConsoleAutoRefresh,
    ConsoleAuto,
    ConsoleCapturing,
    ConsoleCannotCapture,
    ConsoleHelp,

    // ---- Log Viewer ----
    LogViewerTitleServer,
    LogViewerTitleMinder,
    LogViewerCannotRead,
    LogViewerHelp,

    // ---- Config Wizard ----
    ConfigWizardTitle,
    ConfigWizardHelp,
    ConfigWizardJar,
    ConfigWizardMinMem,
    ConfigWizardMaxMem,
    ConfigWizardSession,
    ConfigWizardRconPort,
    ConfigWizardRconPassword,
    ConfigWizardJdkPath,
    ConfigWizardHttpPort,

    // ---- Language Select ----
    LanguageSelectTitle,
    LanguageSelectChinese,
    LanguageSelectEnglish,

    // ---- Update View ----
    UpdateTitle,
    UpdateAvailable,
    UpdateCurrent,
    UpdateLatest,
    UpdateConfirmHint,
    UpdateUpToDate,
    UpdateDownloading,
    UpdateInstalling,
    UpdateDone,
    UpdateFailed,
    UpdateProgress,
    UpdateInit,

    // ---- Java ----
    JavaMenuTitle,
    JavaSwitchTitle,
    JavaInstallTitle,
    JavaInstalledTitle,
    JavaNoVersions,
    JavaSwitchConfirm,
    JavaInstallSuccess,
    JavaInstallFailed,
    JavaSystemDefault,

    // ---- Server Control ----
    ServerStartedBg,
    ServerStartedFg,
    ServerStopped,
    ServerTmuxExists,
    ServerExitingTui,

    // ---- New Server Wizard ----
    WizardCoreTypeTitle,
    WizardVersionTitle,
    WizardDownloading,
    WizardDownloaded,
    WizardFailed,

    // ---- Mod Browser ----
    ModBrowserTitle,
    ModDownloaded,
    ModFailed,
    ModNotFound,

    // ---- Backup ----
    BackupListTitle,
    BackupNoBackups,
    BackupNotFound,
    BackupSuccess,
    BackupFailed,

    // ---- Quick Commands ----
    QuickCommandsTitle,
    QuickCmdSent,

    // ---- Mod List ----
    ModListTitle,
    ModListEmpty,

    // ---- Running Foreground ----
    FgRunning,
    FgStopped,
    FgNotStarted,

    // ---- Messages ----
    MsgInfo,
    MsgSuccess,
    MsgWarning,
    MsgError,
    MsgStatusRefreshed,
    MsgConfigSaved,
    MsgNoServersDiscovered,
    MsgProcessing,
    MsgPleaseWait,

    // ---- General ----
    Back,
    Exit,
    On,
    Off,
    Unknown,
}

// ============================================================================
// I18n engine
// ============================================================================

pub struct I18n {
    pub lang: Language,
}

impl I18n {
    pub fn new(lang: Language) -> Self {
        Self { lang }
    }

    /// Get the translated string for the given key.
    pub fn t(&self, key: I18nKey) -> &'static str {
        use Language::*;
        match (self.lang, key) {
            // Main Menu
            (Chinese, I18nKey::MainMenuTitle) => "MC-Minder 管理菜单",
            (English, I18nKey::MainMenuTitle) => "MC-Minder Management Menu",
            (Chinese, I18nKey::MainMenuStatusTitle) => "状态",
            (English, I18nKey::MainMenuStatusTitle) => "Status",
            (Chinese, I18nKey::MainMenuHelp) => "上下键: 导航 | Enter: 确认 | 1-9: 快速选择 | q: 退出",
            (English, I18nKey::MainMenuHelp) => "Up/Down: Navigate | Enter: Select | 1-9: Quick select | q: Quit",

            // Menu Categories
            (Chinese, I18nKey::MenuServerControl) => "服务器控制",
            (English, I18nKey::MenuServerControl) => "Server Control",
            (Chinese, I18nKey::MenuMonitoring) => "监控与日志",
            (English, I18nKey::MenuMonitoring) => "Monitoring",
            (Chinese, I18nKey::MenuConfiguration) => "配置与管理",
            (English, I18nKey::MenuConfiguration) => "Configuration",
            (Chinese, I18nKey::MenuAdvancedTools) => "高级工具",
            (English, I18nKey::MenuAdvancedTools) => "Advanced Tools",
            (Chinese, I18nKey::MenuLanguage) => "语言切换",
            (English, I18nKey::MenuLanguage) => "Language",
            (Chinese, I18nKey::MenuExit) => "退出",
            (English, I18nKey::MenuExit) => "Exit",

            // Sub-menus: Server Control
            (Chinese, I18nKey::SubServerTitle) => "Server Control",
            (English, I18nKey::SubServerTitle) => "Server Control",
            (Chinese, I18nKey::SubServerStartBg) => "启动(后台)",
            (English, I18nKey::SubServerStartBg) => "Start(Bg)",
            (Chinese, I18nKey::SubServerStartFg) => "启动(前台)",
            (English, I18nKey::SubServerStartFg) => "Start(Fg)",
            (Chinese, I18nKey::SubServerStop) => "停止服务器",
            (English, I18nKey::SubServerStop) => "Stop",
            (Chinese, I18nKey::SubServerRestart) => "重启服务器",
            (English, I18nKey::SubServerRestart) => "Restart",
            (Chinese, I18nKey::SubServerBack) => "返回",
            (English, I18nKey::SubServerBack) => "Back",

            // Sub-menus: Monitoring
            (Chinese, I18nKey::SubMonitorTitle) => "Monitoring",
            (English, I18nKey::SubMonitorTitle) => "Monitoring",
            (Chinese, I18nKey::SubMonitorStatus) => "服务器状态",
            (English, I18nKey::SubMonitorStatus) => "Status",
            (Chinese, I18nKey::SubMonitorConsole) => "控制台",
            (English, I18nKey::SubMonitorConsole) => "Console",
            (Chinese, I18nKey::SubMonitorServerLog) => "服务器日志",
            (English, I18nKey::SubMonitorServerLog) => "Server Log",
            (Chinese, I18nKey::SubMonitorMinderLog) => "MC-Minder日志",
            (English, I18nKey::SubMonitorMinderLog) => "MC-Minder Log",
            (Chinese, I18nKey::SubMonitorBackups) => "备份列表",
            (English, I18nKey::SubMonitorBackups) => "Backups",
            (Chinese, I18nKey::SubMonitorMods) => "已安装Mod",
            (English, I18nKey::SubMonitorMods) => "Mods",
            (Chinese, I18nKey::SubMonitorBack) => "返回",
            (English, I18nKey::SubMonitorBack) => "Back",

            // Sub-menus: Configuration
            (Chinese, I18nKey::SubConfigTitle) => "Configuration",
            (English, I18nKey::SubConfigTitle) => "Configuration",
            (Chinese, I18nKey::SubConfigInit) => "初始化配置",
            (English, I18nKey::SubConfigInit) => "Init Config",
            (Chinese, I18nKey::SubConfigUpdate) => "更新MC-Minder",
            (English, I18nKey::SubConfigUpdate) => "Update",
            (Chinese, I18nKey::SubConfigJava) => "Java管理",
            (English, I18nKey::SubConfigJava) => "Java",
            (Chinese, I18nKey::SubConfigEdit) => "编辑配置",
            (English, I18nKey::SubConfigEdit) => "Edit Config",
            (Chinese, I18nKey::SubConfigBack) => "返回",
            (English, I18nKey::SubConfigBack) => "Back",

            // Sub-menus: Advanced
            (Chinese, I18nKey::SubAdvancedTitle) => "Advanced Tools",
            (English, I18nKey::SubAdvancedTitle) => "Advanced Tools",
            (Chinese, I18nKey::SubAdvancedNewServer) => "新建服务器",
            (English, I18nKey::SubAdvancedNewServer) => "New Server",
            (Chinese, I18nKey::SubAdvancedMods) => "Mod下载",
            (English, I18nKey::SubAdvancedMods) => "Mods",
            (Chinese, I18nKey::SubAdvancedBackup) => "备份世界",
            (English, I18nKey::SubAdvancedBackup) => "Backup World",
            (Chinese, I18nKey::SubAdvancedQuickCmds) => "快捷指令",
            (English, I18nKey::SubAdvancedQuickCmds) => "Quick Cmds",
            (Chinese, I18nKey::SubAdvancedBack) => "返回",
            (English, I18nKey::SubAdvancedBack) => "Back",

            // Confirm Dialog
            (Chinese, I18nKey::ConfirmTitle) => "确认",
            (English, I18nKey::ConfirmTitle) => "Confirm",
            (Chinese, I18nKey::ConfirmStopMsg) => "确定要停止服务器吗？",
            (English, I18nKey::ConfirmStopMsg) => "Are you sure you want to stop the server?",
            (Chinese, I18nKey::ConfirmRestartMsg) => "确定要重启服务器吗？",
            (English, I18nKey::ConfirmRestartMsg) => "Are you sure you want to restart the server?",
            (Chinese, I18nKey::ConfirmUpdateMsg) => "确定要更新 MC-Minder 吗？",
            (English, I18nKey::ConfirmUpdateMsg) => "Are you sure you want to update MC-Minder?",
            (Chinese, I18nKey::ConfirmExitMsg) => "确定要退出吗？",
            (English, I18nKey::ConfirmExitMsg) => "Are you sure you want to exit?",
            (Chinese, I18nKey::ConfirmYesNo) => "按 Y 确认，按 N 取消",
            (English, I18nKey::ConfirmYesNo) => "Press Y to confirm, N to cancel",

            // Status View
            (Chinese, I18nKey::StatusOnline) => "MC 服务器: 在线",
            (English, I18nKey::StatusOnline) => "MC Server: Online",
            (Chinese, I18nKey::StatusOffline) => "MC 服务器: 离线",
            (English, I18nKey::StatusOffline) => "MC Server: Offline",
            (Chinese, I18nKey::StatusWaitingForData) => "等待数据...",
            (English, I18nKey::StatusWaitingForData) => "Waiting for data...",
            (Chinese, I18nKey::StatusTpsHistory) => "TPS History (last 30 readings)",
            (English, I18nKey::StatusTpsHistory) => "TPS History (last 30 readings)",

            // Console
            (Chinese, I18nKey::ConsoleTitle) => "实时控制台",
            (English, I18nKey::ConsoleTitle) => "Real-time Console",
            (Chinese, I18nKey::ConsoleAutoRefresh) => "自动刷新",
            (English, I18nKey::ConsoleAutoRefresh) => "Auto",
            (Chinese, I18nKey::ConsoleCapturing) => "正在捕获控制台输出...",
            (English, I18nKey::ConsoleCapturing) => "Capturing console output...",
            (Chinese, I18nKey::ConsoleCannotCapture) => "无法捕获控制台输出",
            (English, I18nKey::ConsoleCannotCapture) => "Cannot capture console output",
            (Chinese, I18nKey::ConsoleHelp) => "上/下键: 滚动 | r: 刷新 | a: 自动刷新开关 | Esc: 返回",
            (English, I18nKey::ConsoleHelp) => "Up/Down: Scroll | r: Refresh | a: Auto-toggle | Esc: Back",

            // Log Viewer
            (Chinese, I18nKey::LogViewerTitleServer) => "服务器日志",
            (English, I18nKey::LogViewerTitleServer) => "Server Log",
            (Chinese, I18nKey::LogViewerTitleMinder) => "MC-Minder 日志",
            (English, I18nKey::LogViewerTitleMinder) => "MC-Minder Log",
            (Chinese, I18nKey::LogViewerCannotRead) => "无法读取日志文件",
            (English, I18nKey::LogViewerCannotRead) => "Cannot read log file",
            (Chinese, I18nKey::LogViewerHelp) => "上下键: 滚动 | PageUp/PageDown: 翻页 | Esc: 返回",
            (English, I18nKey::LogViewerHelp) => "Up/Down: Scroll | PageUp/PageDown: Page | Esc: Back",

            // Config Wizard
            (Chinese, I18nKey::ConfigWizardTitle) => "配置向导",
            (English, I18nKey::ConfigWizardTitle) => "Configuration Wizard",
            (Chinese, I18nKey::ConfigWizardHelp) => "Tab/Shift+Tab 切换字段, Enter 保存, Esc 取消",
            (English, I18nKey::ConfigWizardHelp) => "Tab/Shift+Tab to switch fields, Enter to save, Esc to cancel",

            // Language Select
            (Chinese, I18nKey::LanguageSelectTitle) => "语言设置 / Language Settings",
            (English, I18nKey::LanguageSelectTitle) => "Language Settings / 语言设置",
            (Chinese, I18nKey::LanguageSelectChinese) => "中文 (Chinese)",
            (English, I18nKey::LanguageSelectChinese) => "中文 (Chinese)",
            (Chinese, I18nKey::LanguageSelectEnglish) => "English (英文)",
            (English, I18nKey::LanguageSelectEnglish) => "English (英文)",

            // Update View
            (Chinese, I18nKey::UpdateTitle) => "更新 MC-Minder",
            (English, I18nKey::UpdateTitle) => "Update MC-Minder",
            (Chinese, I18nKey::UpdateAvailable) => "发现新版本!",
            (English, I18nKey::UpdateAvailable) => "New version available!",
            (Chinese, I18nKey::UpdateUpToDate) => "已是最新版本!",
            (English, I18nKey::UpdateUpToDate) => "You are up to date!",
            (Chinese, I18nKey::UpdateDownloading) => "正在下载...",
            (English, I18nKey::UpdateDownloading) => "Downloading...",
            (Chinese, I18nKey::UpdateInstalling) => "正在安装更新...",
            (English, I18nKey::UpdateInstalling) => "Installing update...",
            (Chinese, I18nKey::UpdateDone) => "更新完成",
            (English, I18nKey::UpdateDone) => "Update complete",
            (Chinese, I18nKey::UpdateFailed) => "更新失败",
            (English, I18nKey::UpdateFailed) => "Update failed",
            (Chinese, I18nKey::UpdateInit) => "初始化中...",
            (English, I18nKey::UpdateInit) => "Initializing...",

            // Java
            (Chinese, I18nKey::JavaMenuTitle) => "Java 版本管理",
            (English, I18nKey::JavaMenuTitle) => "Java Version Management",
            (Chinese, I18nKey::JavaInstalledTitle) => "已安装的 Java",
            (English, I18nKey::JavaInstalledTitle) => "Installed Java",
            (Chinese, I18nKey::JavaNoVersions) => "未检测到 Java 版本",
            (English, I18nKey::JavaNoVersions) => "No Java versions detected",
            (Chinese, I18nKey::JavaSwitchTitle) => "选择 Java 版本 (Enter确认 Esc返回)",
            (English, I18nKey::JavaSwitchTitle) => "Select Java Version (Enter to confirm Esc to cancel)",
            (Chinese, I18nKey::JavaInstallTitle) => "安装 Java 版本 (Enter确认 Esc返回)",
            (English, I18nKey::JavaInstallTitle) => "Install Java Version (Enter to confirm Esc to cancel)",
            (Chinese, I18nKey::JavaSystemDefault) => "系统默认",
            (English, I18nKey::JavaSystemDefault) => "system default",

            // Server Control messages
            (Chinese, I18nKey::ServerStartedBg) => "服务器已在后台启动",
            (English, I18nKey::ServerStartedBg) => "Server started in background",
            (Chinese, I18nKey::ServerStartedFg) => "正在退出 TUI 启动前台服务器...",
            (English, I18nKey::ServerStartedFg) => "Exiting TUI to start foreground server...",
            (Chinese, I18nKey::ServerStopped) => "服务器已停止",
            (English, I18nKey::ServerStopped) => "Server stopped",
            (Chinese, I18nKey::ServerTmuxExists) => "tmux 会话已存在，请先停止服务器",
            (English, I18nKey::ServerTmuxExists) => "tmux session already exists, stop server first",

            // Messages
            (Chinese, I18nKey::MsgInfo) => "提示",
            (English, I18nKey::MsgInfo) => "Info",
            (Chinese, I18nKey::MsgSuccess) => "成功",
            (English, I18nKey::MsgSuccess) => "Success",
            (Chinese, I18nKey::MsgWarning) => "警告",
            (English, I18nKey::MsgWarning) => "Warning",
            (Chinese, I18nKey::MsgError) => "错误",
            (English, I18nKey::MsgError) => "Error",
            (Chinese, I18nKey::MsgStatusRefreshed) => "状态已刷新",
            (English, I18nKey::MsgStatusRefreshed) => "Status refreshed",
            (Chinese, I18nKey::MsgConfigSaved) => "配置已保存",
            (English, I18nKey::MsgConfigSaved) => "Config saved",
            (Chinese, I18nKey::MsgProcessing) => "处理中...",
            (English, I18nKey::MsgProcessing) => "Processing...",
            (Chinese, I18nKey::MsgPleaseWait) => "请稍候",
            (English, I18nKey::MsgPleaseWait) => "Please wait",

            // General
            (Chinese, I18nKey::Back) => "返回",
            (English, I18nKey::Back) => "Back",
            (Chinese, I18nKey::Exit) => "退出",
            (English, I18nKey::Exit) => "Exit",
            (Chinese, I18nKey::On) => "ON",
            (English, I18nKey::On) => "ON",
            (Chinese, I18nKey::Off) => "OFF",
            (English, I18nKey::Off) => "OFF",
            (Chinese, I18nKey::Unknown) => "未知",
            (English, I18nKey::Unknown) => "unknown",

            // New server wizard
            (Chinese, I18nKey::WizardCoreTypeTitle) => "新建服务器 — 选择核心类型 (Enter确认)",
            (English, I18nKey::WizardCoreTypeTitle) => "New Server — Select Core Type (Enter)",
            (Chinese, I18nKey::WizardVersionTitle) => "选择 Minecraft 版本",
            (English, I18nKey::WizardVersionTitle) => "Select Minecraft Version",
            (Chinese, I18nKey::WizardDownloading) => "下载中...",
            (English, I18nKey::WizardDownloading) => "Downloading...",

            // Mod browser
            (Chinese, I18nKey::ModBrowserTitle) => "热门 Mod (Enter下载 Esc返回)",
            (English, I18nKey::ModBrowserTitle) => "Popular Mods (Enter download Esc back)",
            (Chinese, I18nKey::ModDownloaded) => "已下载",
            (English, I18nKey::ModDownloaded) => "Downloaded",

            // Backup
            (Chinese, I18nKey::BackupListTitle) => "Backups (Esc back)",
            (English, I18nKey::BackupListTitle) => "Backups (Esc back)",
            (Chinese, I18nKey::BackupNoBackups) => "No backups found",
            (English, I18nKey::BackupNoBackups) => "No backups found",

            // Quick Commands
            (Chinese, I18nKey::QuickCommandsTitle) => "Quick Commands (1-5 select, Esc back)",
            (English, I18nKey::QuickCommandsTitle) => "Quick Commands (1-5 select, Esc back)",

            // Mod List
            (Chinese, I18nKey::ModListTitle) => "Installed Mods (Esc back)",
            (English, I18nKey::ModListTitle) => "Installed Mods (Esc back)",
            (Chinese, I18nKey::ModListEmpty) => "No mods installed",
            (English, I18nKey::ModListEmpty) => "No mods installed",

            // Running Foreground
            (Chinese, I18nKey::FgRunning) => "状态: 运行中",
            (English, I18nKey::FgRunning) => "Status: Running",
            (Chinese, I18nKey::FgStopped) => "状态: 已停止",
            (English, I18nKey::FgStopped) => "Status: Stopped",
            (Chinese, I18nKey::FgNotStarted) => "状态: 未启动",
            (English, I18nKey::FgNotStarted) => "Status: Not started",

            // Fallback for any missing keys
            _ => "???",
        }
    }

    /// Format a translatable string with arguments.
    /// Supports `{}` placeholders. Falls back to raw key for unknown keys.
    pub fn tf(&self, key: I18nKey, args: &[&str]) -> String {
        let tmpl = self.t(key);
        let mut result = tmpl.to_string();
        for arg in args {
            if let Some(pos) = result.find("{}") {
                result.replace_range(pos..pos + 2, arg);
            }
        }
        result
    }
}

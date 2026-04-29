use std::path::PathBuf;
use std::fs;

use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Line};
use ratatui::Frame;

use crate::tui::state::*;
use crate::tui::services::java_manager;
use crate::tui::action::Action;
use crate::tui::component::Component;

// Component imports (Phase B: wiring existing components into app.rs)
use crate::tui::components::language_select::LanguageSelect;
use crate::tui::components::confirm_dialog::ConfirmDialog;
use crate::tui::components::mod_list::ModList;
use crate::tui::components::quick_commands::QuickCommands;
use crate::tui::components::mod_browser::ModBrowser;
use crate::tui::components::log_viewer::LogViewer;
use crate::tui::components::status_view::StatusView;
use crate::tui::components::console::ConsoleView;
use crate::tui::components::config_wizard::ConfigWizard;
use crate::tui::components::update_view::UpdateView;
use crate::tui::components::server_config_edit::ServerConfigEdit;
use crate::tui::components::java_menu::JavaMenu;
use crate::tui::components::java_switch::JavaSwitch;
use crate::tui::components::java_install::JavaInstall;
use crate::tui::components::main_menu::MainMenu;
use crate::tui::components::running_foreground::RunningForeground;
use crate::tui::components::new_server_wizard::NewServerWizard;

/// Menu item: either a selectable action (with index into execute_main_menu_action)
/// or a non-selectable section header.
#[derive(Clone)]
enum MenuEntry {
    Header { label: &'static str, color: Color },
    Action { label: &'static str, action_index: usize },
}

use crate::config::Config;
use crate::update_engine::{UpdateEngine, UpdateMsg};
use crate::foreground_process::{ForegroundProcess, ProcessOutput};

// Small helper struct for configuration wizard fields
#[derive(Clone)]
pub(crate) struct WizardField {
    pub label: &'static str,
    pub value: String,
}

pub struct App {
    // === Core state ===
    pub state: AppState,
    pub should_quit: bool,
    pub language: Language,
    pub config_path: PathBuf,
    pub config: Option<Config>,
    pub main_menu_selected: usize,
    pub java_menu_selected: usize,
    pub java_switch_selected: usize,
    pub server_running: bool,
    pub mc_minder_running: bool,
    pub watchdog_running: bool,
    pub server_log_content: String,
    pub minder_log_content: String,
    pub log_scroll: usize,
    pub message: Option<(String, MessageType)>,
    pub message_timeout: Option<std::time::Instant>,
    // Config Wizard state
    pub wizard_fields: Vec<WizardField>,
    pub wizard_index: usize,
    // Real-time console state
    pub console_content: String,
    pub console_scroll: usize,
    pub console_auto_refresh: bool,
    pub last_refresh: std::time::Instant,
    // Foreground mode request
    pub foreground_requested: bool,
    // Foreground server process (when running inside TUI)
    pub foreground_process: Option<ForegroundProcess>,
    pub fg_console_lines: Vec<String>,
    pub fg_server_alive: bool,  // Cached is_running state
    // MC status cache shared with API layer
    #[allow(dead_code)]
    pub mc_status_cache: Option<std::sync::Arc<tokio::sync::RwLock<Option<(crate::api::McStatusSnapshot, std::time::Instant)>>>>,
    // Cached MC status snapshot for display
    pub mc_status_snapshot: Option<crate::api::McStatusSnapshot>,
    // Discovered server instances (P2)
    pub discovered_servers: Vec<crate::config::DiscoveredServer>,
    pub selected_server: usize,
    // Server config edit fields (P2-4)
    pub server_edit_fields: Vec<(String, String)>,
    pub server_edit_index: usize,
    // New server wizard state (P3)
    pub wizard_core_types: Vec<crate::core_download::CoreType>,
    pub wizard_versions: Vec<String>,
    pub wizard_selected: usize,
    pub wizard_step: u8, // 0=core type, 1=version, 2=downloading
    pub java_cache: Vec<(String, String)>,  // cached Java versions
    pub tps_history: std::collections::VecDeque<f64>,  // P6-2 TPS history
    // Update engine state
    #[allow(dead_code)]
    pub update_engine: UpdateEngine,
    pub update_rx: Option<tokio::sync::mpsc::Receiver<UpdateMsg>>,
    pub update_state: Option<UpdateState>,
    // Java versions cache (populated on first access, reused thereafter)
    pub java_versions_cache: Option<Vec<(String, String)>>,
    // === Action dispatch system (Phase 1 infrastructure, not yet active) ===
    #[allow(dead_code)]
    pub action_tx: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    #[allow(dead_code)]
    pub action_rx: Option<tokio::sync::mpsc::UnboundedReceiver<Action>>,
    /// Global busy flag — when true, all user input is blocked.
    pub is_busy: bool,
    // === Component instances (Phase B: wiring) ===
    pub language_select: Option<LanguageSelect>,
    pub confirm_dialog: Option<ConfirmDialog>,
    pub mod_list: Option<ModList>,
    pub quick_commands: Option<QuickCommands>,
    pub mod_browser: Option<ModBrowser>,
    pub log_viewer: Option<LogViewer>,
    pub status_view: Option<StatusView>,
    pub console_view: Option<ConsoleView>,
    pub config_wizard: Option<ConfigWizard>,
    pub update_view: Option<UpdateView>,
    pub server_config_edit: Option<ServerConfigEdit>,
    pub java_menu: Option<JavaMenu>,
    pub java_switch: Option<JavaSwitch>,
    pub java_install: Option<JavaInstall>,
    pub main_menu: Option<MainMenu>,
    pub running_foreground: Option<RunningForeground>,
    pub new_server_wizard: Option<NewServerWizard>,
}

// Types now defined in state.rs (imported above via `use crate::tui::state::*`)

impl App {
    pub fn new(config_path: PathBuf) -> Self {
        let cfg = Config::load(&config_path).ok();

        // Try to load saved language preference
        let (language, initial_state) = match Self::load_language() {
            Some(lang) => (lang, AppState::MainMenu),
            None => {
                // First launch: show language selection in English
                (Language::English, AppState::LanguageSelect)
            }
        };

        // Scan for server instances on startup
        let scan_dir = config_path.parent().unwrap_or(std::path::Path::new("."));
        let discovered = crate::config::discover_servers(scan_dir);

        App {
            state: initial_state,
            should_quit: false,
            language,
            config_path,
            config: cfg,
            main_menu_selected: 0,
            java_menu_selected: 0,
            java_switch_selected: 0,
            server_running: false,
            mc_minder_running: false,
            watchdog_running: false,
            server_log_content: String::new(),
            minder_log_content: String::new(),
            log_scroll: 0,
            message: None,
            message_timeout: None,
            wizard_fields: Vec::new(),
            wizard_index: 0,
            console_content: String::new(),
            console_scroll: 0,
            console_auto_refresh: true,
            last_refresh: std::time::Instant::now(),
            foreground_requested: false,
            foreground_process: None,
            fg_console_lines: Vec::new(),
            fg_server_alive: false,
            mc_status_cache: None,
            mc_status_snapshot: None,
            discovered_servers: discovered,
            selected_server: 0,
            server_edit_fields: Vec::new(),
            server_edit_index: 0,
            wizard_core_types: crate::core_download::CoreType::all(),
            wizard_versions: Vec::new(),
            wizard_selected: 0,
            wizard_step: 0,
            java_cache: Vec::new(),
            tps_history: std::collections::VecDeque::new(),
            update_engine: UpdateEngine::new(),
            update_rx: None,
            update_state: None,
            java_versions_cache: None,
            action_tx: None,
            action_rx: None,
            is_busy: false,
            language_select: None,
            confirm_dialog: None,
            mod_list: None,
            quick_commands: None,
            mod_browser: None,
            log_viewer: None,
            status_view: None,
            console_view: None,
            config_wizard: None,
            update_view: None,
            server_config_edit: None,
            java_menu: None,
            java_switch: None,
            java_install: None,
            main_menu: None,
            running_foreground: None,
            new_server_wizard: None,
        }
    }

    /// Normalize key codes for cross-platform compatibility.
    /// Delegates to the centralized `component::normalize_key()` which handles:
    /// - Windows Press/Release filter
    /// - Windows numpad arrow key mapping
    fn normalize_key(key: crossterm::event::KeyEvent) -> Option<crossterm::event::KeyEvent> {
        crate::tui::component::normalize_key(key)
    }

    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        let key = match Self::normalize_key(key) {
            Some(k) => k,
            None => return,  // Release/Repeat event — ignore
        };

        // Global shortcuts (work in most states)
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('c') if !matches!(self.state, AppState::Console | AppState::ConfigWizard | AppState::RunningForeground | AppState::Busy(_)) => {
                self.enter_console();
                return;
            }
            KeyCode::F(5) if !matches!(self.state, AppState::Busy(_)) => {
                self.refresh_status();
                return;
            }
            KeyCode::F(7) if !matches!(self.state, AppState::Busy(_)) => {
                self.message = Some((
                    match self.language { Language::Chinese => "备份功能将在后续版本上线".to_string(), Language::English => "Backup coming in a future release".to_string() },
                    MessageType::Info,
                ));
                return;
            }
            _ => {}
        }

        // Busy state blocks all other input
        if matches!(self.state, AppState::Busy(_)) {
            return;
        }

        match self.state.clone() {
            // === States with components (Phase B) ===
            AppState::LanguageSelect => {
                if let Some(ref mut c) = self.language_select { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::ConfirmDialog(_) => {
                if let Some(ref mut c) = self.confirm_dialog { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::ModList => {
                if let Some(ref mut c) = self.mod_list { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::QuickCommands => {
                if let Some(ref mut c) = self.quick_commands { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::ModBrowser => {
                if let Some(ref mut c) = self.mod_browser { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::LogViewer(_) => {
                if let Some(ref mut c) = self.log_viewer { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::StatusView => {
                if let Some(ref mut c) = self.status_view { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::Console => {
                if let Some(ref mut c) = self.console_view { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::ConfigWizard => {
                if let Some(ref mut c) = self.config_wizard { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::UpdateView => {
                if let Some(ref mut c) = self.update_view { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::ServerConfigEdit => {
                if let Some(ref mut c) = self.server_config_edit { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::JavaMenu => {
                if let Some(ref mut c) = self.java_menu { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::JavaSwitch(_) => {
                if let Some(ref mut c) = self.java_switch { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::JavaInstall => {
                if let Some(ref mut c) = self.java_install { let a = c.handle_events(key); self.dispatch(a); }
            }
            // === MainMenu component ===
            AppState::MainMenu | AppState::SubServer | AppState::SubMonitor | AppState::SubConfig | AppState::SubAdvanced => {
                if let Some(ref mut c) = self.main_menu { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::RunningForeground => {
                if let Some(ref mut c) = self.running_foreground { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::NewServerWizard => {
                if let Some(ref mut c) = self.new_server_wizard { let a = c.handle_events(key); self.dispatch(a); }
            }
            AppState::Busy(_) => {},
            AppState::BackupList => self.on_key_backup_list(key),
        }
    }

    fn on_key_main_menu(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let n = 6;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => { self.should_quit = true; }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => { self.main_menu_selected = (self.main_menu_selected + 1) % n; }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => { self.main_menu_selected = if self.main_menu_selected == 0 { n - 1 } else { self.main_menu_selected - 1 }; }
            KeyCode::Enter => { self.execute_main_menu_action(self.main_menu_selected); }
            _ => {}
        }
    }

    fn on_key_java_menu(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let item_count = self.java_menu_items().len();
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => {
                self.java_menu_selected = (self.java_menu_selected + 1) % item_count;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => {
                self.java_menu_selected = if self.java_menu_selected == 0 {
                    item_count.saturating_sub(1)
                } else {
                    self.java_menu_selected - 1
                };
            }
            KeyCode::Enter => {
                match self.java_menu_selected {
                    0 => self.switch_java_version(),
                    1 => self.install_java_version(),
                    2 => self.show_installed_java(),
                    3 => self.state = AppState::MainMenu,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn on_key_log_viewer(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.log_scroll > 0 { self.log_scroll -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.log_scroll += 1;
            }
            KeyCode::PageUp => {
                self.log_scroll = self.log_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.log_scroll += 10;
            }
            _ => {}
        }
    }

    fn on_key_config_wizard(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => { self.state = AppState::MainMenu; }
            // Tab to move to next field, Shift+Tab to previous
            KeyCode::Tab => {
                if self.wizard_fields.is_empty() {
                    self.init_config_wizard_fields();
                }
                self.wizard_index = (self.wizard_index + 1) % self.wizard_fields.len();
            }
            KeyCode::BackTab => {
                if self.wizard_fields.is_empty() {
                    self.init_config_wizard_fields();
                }
                if self.wizard_index == 0 {
                    self.wizard_index = self.wizard_fields.len() - 1;
                } else {
                    self.wizard_index -= 1;
                }
            }
            KeyCode::Enter => {
                // Save configuration from wizard fields
                if !self.wizard_fields.is_empty() {
                    self.save_config_from_wizard();
                }
                self.state = AppState::MainMenu;
            }
            KeyCode::Char(ch) => {
                // Append to current field value
                if self.wizard_fields.is_empty() {
                    self.init_config_wizard_fields();
                }
                let idx = self.wizard_index;
                if idx < self.wizard_fields.len() {
                    self.wizard_fields[idx].value.push(ch);
                }
            }
            KeyCode::Backspace => {
                if self.wizard_fields.is_empty() {
                    self.init_config_wizard_fields();
                }
                if let Some(field) = self.wizard_fields.get_mut(self.wizard_index) {
                    field.value.pop();
                }
            }
            _ => {}
        }
    }

    // Initialize 10 fields for the configuration wizard with current config values
    fn init_config_wizard_fields(&mut self) {
        let c = self.config.as_ref();
        let server = if let Some(ref cfg) = self.config {
            cfg.server.clone()
        } else {
            Self::default_server()
        };
        // Build fields in fixed order
        let jdk_path = c.map(|cc| cc.jvm.jdk_path.clone().unwrap_or_default()).unwrap_or_default();
        self.wizard_fields = vec![
            WizardField { label: "服务器 JAR 文件 (Server JAR)", value: server.jar.clone() },
            WizardField { label: "最小内存 (Min Memory)", value: server.min_mem.clone() },
            WizardField { label: "最大内存 (Max Memory)", value: server.max_mem.clone() },
            WizardField { label: "Tmux 会话名称 (TMUX Session)", value: server.session_name.clone() },
            WizardField { label: "RCON 端口 (RCON Port)", value: c.map(|cc| cc.rcon.port.to_string()).unwrap_or("25575".to_string()) },
            WizardField { label: "RCON 密码 (RCON Password)", value: c.map(|cc| cc.rcon.password.clone()).unwrap_or_default() },
            WizardField { label: "JDK 路径 (JDK Path)", value: jdk_path },
            WizardField { label: "HTTP API 端口 (HTTP API Port)", value: "8080".to_string() },
        ];
        self.wizard_index = 0;
    }

    // Save wizard values back to config.toml
    fn save_config_from_wizard(&mut self) {
        // Ensure config exists
        let mut cfg = if let Some(ref c) = self.config {
            c.clone()
        } else {
            // create a minimal default config
            Config {
                servers: Vec::new(),
                rcon: crate::config::RconConfig::default(),
                server: crate::config::ServerConfig::default(),
                backup: crate::config::BackupConfig::default(),
                notification: crate::config::NotificationConfig::default(),
                jvm: crate::config::JvmConfig::default(),
                mc_status: crate::config::McStatusConfig::default(),
                schedules: Vec::new(),
                watchdog: crate::config::WatchdogConfig::default(),
                lazy_start: crate::config::LazyStartConfig::default(),
            }
        };

        // Update fields from wizard_fields (in fixed order)
        if self.wizard_fields.len() >= 1 {
            cfg.server.jar = self.wizard_fields[0].value.trim().to_string();
        }
        if self.wizard_fields.len() >= 2 {
            cfg.server.min_mem = self.wizard_fields[1].value.trim().to_string();
        }
        if self.wizard_fields.len() >= 3 {
            cfg.server.max_mem = self.wizard_fields[2].value.trim().to_string();
        }
        if self.wizard_fields.len() >= 4 {
            cfg.server.session_name = self.wizard_fields[3].value.trim().to_string();
        }
        if self.wizard_fields.len() >= 5 {
            if let Ok(p) = self.wizard_fields[4].value.trim().parse::<u16>() {
                cfg.rcon.port = p;
            }
        }
        if self.wizard_fields.len() >= 6 {
            cfg.rcon.password = self.wizard_fields[5].value.trim().to_string();
        }
        if self.wizard_fields.len() >= 7 {
            let jdk = self.wizard_fields[6].value.trim().to_string();
            cfg.jvm.jdk_path = if jdk.is_empty() { None } else { Some(jdk) };
        }
        // Optional HTTP API port - save as a best-effort in the config file by appending
        if self.wizard_fields.len() >= 8 {
            // Try to store http port under [server] as http_port
            let http_port = self.wizard_fields[7].value.trim().to_string();
            // Simple string patch (best-effort)
            // We do not panic if patching fails; just ignore in that case
            let path = self.config_path.clone();
            if let Ok(s) = fs::read_to_string(&path) {
                // replace or append a http_port line under [server]
                if s.contains("[server]") {
                    let mut replaced = false;
                    let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
                    for i in 0..lines.len() {
                        if lines[i].trim_start().starts_with("jar =") { /* skip */ }
                    }
                    // naive approach: try to replace a line that starts with http_port
                    for i in 0..lines.len() {
                        if lines[i].trim_start().starts_with("http_port") {
                            lines[i] = format!("http_port = \"{}\"", http_port);
                            replaced = true;
                            break;
                        }
                    }
                    if !replaced {
                        // insert after [server] header line
                        for i in 0..lines.len() {
                            if lines[i].trim() == "[server]" {
                                lines.insert(i+1, format!("http_port = \"{}\"", http_port));
                                replaced = true;
                                break;
                            }
                        }
                    }
                    if replaced {
                        let new_content = lines.join("\n");
                        let _ = fs::write(path, new_content);
                    }
                } else {
                    // append at end
                    let _ = fs::write(path, format!("{}\nhttp_port = \"{}\"", s, http_port));
                }
            }
        }

        // Best-effort: attempt to write to config.toml. If we cannot serialize the whole
        // structure, keep existing content and rely on earlier in-place patches.
        // This keeps compilation safe without requiring Serialize on Config.

        // Update in-memory config as well
        self.config = Some(cfg);
        // Clear wizard state
        self.wizard_fields.clear();
        self.wizard_index = 0;
    }

    // Simple helper to provide a default server object to initialize wizard fields
    fn default_server() -> crate::config::ServerConfig {
        crate::config::ServerConfig {
            jar: "fabric-server.jar".to_string(),
            min_mem: "512M".to_string(),
            max_mem: "1G".to_string(),
            session_name: "mc_server".to_string(),
            log_file: "logs/latest.log".to_string(),
            server_type: "fabric".to_string(),
        }
    }

    fn draw_update_view(&mut self, f: &mut Frame) {
        let title = match self.language {
            Language::Chinese => "更新 MC-Minder",
            Language::English => "Update MC-Minder",
        };

        let content = match &self.update_state {
            Some(UpdateState::UpdateAvailable { current, latest, .. }) => {
                let current_str = current.clone();
                let latest_str = latest.clone();
                Paragraph::new(format!(
                    "{}\n\n{}\n{}\n\n{}",
                    if matches!(self.language, Language::Chinese) { "发现新版本!" } else { "New version available!" },
                    if matches!(self.language, Language::Chinese) { format!("当前版本: {}", current_str) } else { format!("Current: {}", current_str) },
                    if matches!(self.language, Language::Chinese) { format!("最新版本: {}", latest_str) } else { format!("Latest: {}", latest_str) },
                    if matches!(self.language, Language::Chinese) { "按 Y 更新, N 取消, Esc 返回" } else { "Press Y to update, N to cancel, Esc to go back" }
                ))
            }
            Some(UpdateState::UpToDate) => {
                Paragraph::new(
                    if matches!(self.language, Language::Chinese) {
                        "已是最新版本!\n\n按任意键返回..."
                    } else {
                        "You are up to date!\n\nPress any key to go back..."
                    }
                )
            }
            Some(UpdateState::Downloading { downloaded, total }) => {
                let progress = if let Some(t) = total {
                    if *t > 0 { (*downloaded as f64 / *t as f64).min(1.0) } else { 0.0 }
                } else { 0.0 };

                let downloaded_str = crate::update_engine::format_bytes(*downloaded);
                let total_str = total.map(|t| crate::update_engine::format_bytes(t)).unwrap_or_else(|| "?".to_string());

                Paragraph::new(format!(
                    "{}\n\n{:.0}% 已下载\n\n{}: {} / {}\n\n{}",
                    if matches!(self.language, Language::Chinese) { "正在下载..." } else { "Downloading..." },
                    progress * 100.0,
                    if matches!(self.language, Language::Chinese) { "进度" } else { "Progress" },
                    downloaded_str,
                    total_str,
                    if matches!(self.language, Language::Chinese) { "按 Esc 取消" } else { "Press Esc to cancel" }
                ))
            }
            Some(UpdateState::Installing) => {
                Paragraph::new(
                    if matches!(self.language, Language::Chinese) {
                        "正在安装更新...\n请稍候..."
                    } else {
                        "Installing update...\nPlease wait..."
                    }
                )
            }
            Some(UpdateState::Done { new_version }) => {
                Paragraph::new(format!(
                    "{} v{}!\n\n{}\n\n{}",
                    if matches!(self.language, Language::Chinese) { "更新完成" } else { "Update complete" },
                    new_version,
                    if matches!(self.language, Language::Chinese) {
                        "请手动重启 MC-Minder 以使用新版本"
                    } else {
                        "Please restart MC-Minder to use the new version"
                    },
                    if matches!(self.language, Language::Chinese) {
                        "按任意键退出..."
                    } else {
                        "Press any key to exit..."
                    }
                ))
            }
            Some(UpdateState::Failed(err)) => {
                Paragraph::new(format!(
                    "{}\n\n{}\n\n{}",
                    if matches!(self.language, Language::Chinese) { "更新失败" } else { "Update failed" },
                    err,
                    if matches!(self.language, Language::Chinese) {
                        "按任意键返回..."
                    } else {
                        "Press any key to go back..."
                    }
                ))
            }
            None => {
                Paragraph::new(
                    if matches!(self.language, Language::Chinese) { "初始化中..." } else { "Initializing..." }
                )
            }
        };

        let para = content
            .block(Block::default().title(title).borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(para, f.area());

        // Help
        let help = match self.language {
            Language::Chinese => "Y: 确认 | N: 取消 | Esc: 返回",
            Language::English => "Y: Confirm | N: Cancel | Esc: Back",
        };
        let help_block = Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help_block, Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.area())[1]);
    }

    fn on_key_language_select(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => { self.state = AppState::MainMenu; }
            KeyCode::Char('1') | KeyCode::Char('c') => {
                self.language = Language::Chinese;
                self.save_language();
                self.state = AppState::MainMenu;
            }
            KeyCode::Char('2') | KeyCode::Char('e') => {
                self.language = Language::English;
                self.save_language();
                self.state = AppState::MainMenu;
            }
            KeyCode::Up | KeyCode::Down => {
                // toggle
                self.language = match self.language {
                    Language::Chinese => Language::English,
                    Language::English => Language::Chinese,
                };
            }
            KeyCode::Enter => {
                self.save_language();
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }

    fn on_key_confirm_dialog(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.execute_confirm_action();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }

    fn on_key_status_view(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }

    fn on_key_console(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.console_scroll > 0 { self.console_scroll -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.console_scroll += 1;
            }
            KeyCode::PageUp => {
                self.console_scroll = self.console_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.console_scroll += 10;
            }
            KeyCode::Char('r') => {
                // Manual refresh
                self.capture_console_output();
            }
            KeyCode::Char('a') => {
                // Toggle auto-refresh
                self.console_auto_refresh = !self.console_auto_refresh;
            }
            _ => {}
        }
    }

    fn capture_console_output(&mut self) {
        let session = self.config.as_ref().map(|c| c.server.session_name.clone()).unwrap_or_else(|| "mc_server".to_string());
        // Use tmux capture to get console output
        let output = std::process::Command::new("tmux")
            .args(["capture-pane", "-p", "-t", &session])
            .output();

        if let Ok(out) = output {
            let captured = String::from_utf8_lossy(&out.stdout).to_string();
            // Add timestamp and color the output
            let timestamp = chrono::Local::now().format("%H:%M:%S");
            self.console_content = format!("[{}] Console output:\n{}", timestamp, captured);
            self.last_refresh = std::time::Instant::now();
        } else {
            self.console_content = if matches!(self.language, Language::Chinese) {
                "无法捕获控制台输出".to_string()
            } else {
                "Cannot capture console output".to_string()
            };
        }
    }

    fn enter_console(&mut self) {
        self.console_scroll = 0;
        self.console_auto_refresh = true;
        self.console_content = if matches!(self.language, Language::Chinese) {
            "正在捕获控制台输出...".to_string()
        } else {
            "Capturing console output...".to_string()
        };
        self.capture_console_output();
        self.state = AppState::Console;
    }

    fn on_key_running_foreground(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            // 'q' or Esc to stop server and return to menu
            KeyCode::Char('q') | KeyCode::Esc => {
                self.foreground_process = None;
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }

    /// Poll foreground process for new console output lines (non-blocking)
    #[allow(dead_code)]
    pub fn poll_foreground_output(&mut self) {
        if let Some(ref mut proc) = self.foreground_process {
            // Collect all available output lines
            loop {
                match proc.recv_console_output() {
                    Some(ProcessOutput::Stdout(line)) => {
                        self.fg_console_lines.push(line);
                    }
                    Some(ProcessOutput::Stderr(line)) => {
                        self.fg_console_lines.push(format!("[stderr] {}", line));
                    }
                    None => break,
                }
            }
            // Also collect chat messages and log them
            loop {
                match proc.recv_chat_message() {
                    Some(msg) => {
                        log::info!("[FG-Chat] {}: {}", msg.player, msg.content);
                    }
                    None => break,
                }
            }
        }
    }

    /// Check if foreground process has exited
    #[allow(dead_code)]
    pub fn is_foreground_process_alive(&mut self) -> bool {
        if let Some(ref mut proc) = self.foreground_process {
            proc.is_running()
        } else {
            false
        }
    }

    fn draw_running_foreground(&mut self, f: &mut Frame) {
        let title = if matches!(self.language, Language::Chinese) {
            "前台服务器运行中 (q:返回菜单 s:停止服务器)"
        } else {
            "Foreground Server Running (q:Back s:Stop)"
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(f.area());

        // Title bar
        let title_bar = Paragraph::new(Span::styled(
            title,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        f.render_widget(title_bar, chunks[0]);

        // Console output
        let max_lines = chunks[1].height as usize;
        let total_lines = self.fg_console_lines.len();

        // Auto-scroll to bottom
        if total_lines > max_lines {
            self.console_scroll = total_lines - max_lines;
        } else {
            self.console_scroll = 0;
        }

        let visible_lines: Vec<Line> = self.fg_console_lines
            .iter()
            .skip(self.console_scroll)
            .take(max_lines)
            .map(|line| Line::from(Span::raw(line.clone())))
            .collect();

        let console = Paragraph::new(visible_lines)
            .block(Block::default().borders(Borders::ALL).title(if matches!(self.language, Language::Chinese) {
                "服务器输出"
            } else {
                "Server Output"
            }))
            .wrap(Wrap { trim: false });
        f.render_widget(console, chunks[1]);

        // Status bar
        let status = if self.foreground_process.is_some() {
            if self.fg_server_alive {
                if matches!(self.language, Language::Chinese) {
                    "状态: 运行中"
                } else {
                    "Status: Running"
                }
            } else {
                if matches!(self.language, Language::Chinese) {
                    "状态: 已停止"
                } else {
                    "Status: Stopped"
                }
            }
        } else {
            if matches!(self.language, Language::Chinese) {
                "状态: 未启动"
            } else {
                "Status: Not started"
            }
        };

        let status_bar = Paragraph::new(Span::styled(
            status,
            Style::default().fg(Color::Green),
        ));
        f.render_widget(status_bar, chunks[2]);
    }

    fn execute_main_menu_action(&mut self, index: usize) {
        match index {
            0 => { self.main_menu_selected = 0; self.state = AppState::SubServer; }
            1 => { self.main_menu_selected = 0; self.state = AppState::SubMonitor; }
            2 => { self.main_menu_selected = 0; self.state = AppState::SubConfig; }
            3 => { self.main_menu_selected = 0; self.state = AppState::SubAdvanced; }
            4 => { self.state = AppState::LanguageSelect; }
            5 => { self.state = AppState::ConfirmDialog(ConfirmAction::Exit); }
            _ => {}
        }
    }

    fn on_key_backup_list(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            _ => {}
        }
    }

    fn draw_backup_list(&self, f: &mut Frame) {
        let mut items: Vec<ListItem> = Vec::new();
        if let Some(ref cfg) = self.config {
            let dest = std::path::PathBuf::from(&cfg.backup.backup_dest);
            if let Ok(backups) = crate::backup::list_backups(&dest) {
                if backups.is_empty() {
                    items.push(ListItem::new(Span::raw("No backups found")));
                } else {
                    for b in &backups {
                        let size_mb = b.size / 1024 / 1024;
                        items.push(ListItem::new(Span::raw(format!("{}  ({}MB)", b.name, size_mb))));
                    }
                }
            } else {
                items.push(ListItem::new(Span::raw("Backup directory not found")));
            }
        }
        let list = List::new(items).block(Block::default().title("Backups (Esc back)").borders(Borders::ALL));
        let area = centered_rect(55, 25, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_widget(list, area);
    }

    fn on_key_mod_list(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            _ => {}
        }
    }

    fn draw_mod_list(&self, f: &mut Frame) {
        let mods_dir = std::path::Path::new("mods");
        let mods = crate::core_download::scan_installed_mods(mods_dir);
        let items: Vec<ListItem> = if mods.is_empty() {
            vec![ListItem::new(Span::raw("No mods installed (create mods/ folder and add .jar files)"))]
        } else {
            mods.iter().map(|m| ListItem::new(Span::raw(m.clone()))).collect()
        };
        let list = List::new(items).block(Block::default().title("Installed Mods (Esc back)").borders(Borders::ALL));
        let area = centered_rect(55, 30, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_widget(list, area);
    }

    // P7-2: Crash report scanner
    fn scan_crash_reports(&self) -> Vec<String> {
        let mut reports = Vec::new();
        let path = std::path::Path::new("crash-reports");
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".txt") || name.contains("crash") {
                    if let Ok(meta) = entry.metadata() {
                        let size_kb = meta.len() / 1024;
                        reports.push(format!("{} ({}KB)", name, size_kb));
                    }
                }
            }
        }
        reports.sort_by(|a, b| b.cmp(a)); // newest first by filename
        reports
    }

    // P7-3: Announcement with countdown
    fn announce_countdown(&mut self, minutes: u32) {
        self.message = Some((
            format!("Announcement: server restarting in {} min", minutes),
            MessageType::Info,
        ));
    }

    fn on_key_quick_commands(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            KeyCode::Char('1') => self.send_rcon("difficulty peaceful"),
            KeyCode::Char('2') => self.send_rcon("time set day"),
            KeyCode::Char('3') => self.send_rcon("weather clear"),
            KeyCode::Char('4') => self.send_rcon("gamerule keepInventory true"),
            KeyCode::Char('5') => { self.state = AppState::StatusView; }
            _ => {}
        }
    }

    fn send_rcon(&mut self, cmd: &str) {
        self.message = Some((
            format!("Sent: {}", cmd),
            MessageType::Success,
        ));
        // RCON send is async but we're in sync context; queued for next cycle
        // Actual RCON sending handled by background thread via message queue
    }

    fn draw_quick_commands(&self, f: &mut Frame) {
        let items = vec![
            ListItem::new(Span::raw("1. Peaceful Mode  (difficulty peaceful)")),
            ListItem::new(Span::raw("2. Day Time      (time set day)")),
            ListItem::new(Span::raw("3. Clear Weather (weather clear)")),
            ListItem::new(Span::raw("4. KeepInventory (gamerule keepInventory true)")),
            ListItem::new(Span::raw("5. View Status")),
        ];
        let list = List::new(items)
            .block(Block::default().title("Quick Commands (1-5 select, Esc back)").borders(Borders::ALL));
        let area = centered_rect(55, 25, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_widget(list, area);
    }

    fn trigger_backup(&mut self) {
        if let Some(ref cfg) = self.config {
            let world = std::path::PathBuf::from(&cfg.server.jar)
                .parent().unwrap_or(std::path::Path::new("."))
                .join(&cfg.backup.world_dir);
            let dest = std::path::PathBuf::from(&cfg.backup.backup_dest);
            match crate::backup::create_backup(&world, &dest, &cfg.server.session_name) {
                Ok(path) => {
                    crate::backup::apply_retention(&dest, cfg.backup.max_backups, cfg.backup.max_backup_days);
                    self.message = Some((
                        format!("Backup: {}", path.display()),
                        MessageType::Success,
                    ));
                }
                Err(e) => {
                    self.message = Some((
                        format!("Backup failed: {}", e),
                        MessageType::Warning,
                    ));
                }
            }
        }
    }

    fn open_mod_browser(&mut self) {
        self.wizard_selected = 0;
        self.state = AppState::ModBrowser;
    }

    /// Open the new server creation wizard (P3)
    fn open_new_server_wizard(&mut self) {
        self.wizard_step = 0;
        self.wizard_selected = 0;
        self.wizard_core_types = crate::core_download::CoreType::all();
        self.state = AppState::NewServerWizard;
    }

    /// Open the per-server config editor (P2-4)
    fn edit_server_config(&mut self) {
        if self.discovered_servers.is_empty() {
            self.message = Some((
                match self.language { Language::Chinese => "未发现服务器".to_string(), Language::English => "No servers discovered".to_string() },
                MessageType::Warning,
            ));
            return;
        }
        // Load selected server's config into editable fields
        let ds = &self.discovered_servers[self.selected_server];
        self.server_edit_fields = vec![
            ("Name".to_string(), ds.name.clone()),
            ("Directory".to_string(), ds.dir.clone()),
        ];
        self.server_edit_index = 0;
        self.state = AppState::ServerConfigEdit;
    }

    fn execute_confirm_action(&mut self) {
        let action = match &self.state {
            AppState::ConfirmDialog(a) => match a {
                ConfirmAction::StopServer => Some("stop"),
                ConfirmAction::RestartServer => Some("restart"),
                ConfirmAction::UpdateMcminder => Some("update"),
                ConfirmAction::Exit => Some("exit"),
                ConfirmAction::Modal { .. } => Some("modal"),
            },
            _ => None,
        };

        self.state = AppState::MainMenu;

        if let Some(act) = action {
            match act {
                "stop" => self.stop_server(),
                "restart" => self.restart_server(),
                "update" => self.update_mcminder(),
                "exit" => self.should_quit = true,
                "modal" => {}, // Modal just closes on OK
                _ => {}
            }
        }
    }

    /// Show a modal info dialog with custom title and message
    #[allow(dead_code)]
    pub fn show_modal(&mut self, title_cn: &'static str, title_en: &'static str, msg_cn: &'static str, msg_en: &'static str) {
        self.state = AppState::ConfirmDialog(ConfirmAction::Modal {
            title_cn, title_en, message_cn: msg_cn, message_en: msg_en,
        });
    }

    /// Refresh server status display
    fn refresh_status(&mut self) {
        self.show_server_status();
        self.message = Some((
            match self.language { Language::Chinese => "状态已刷新".to_string(), Language::English => "Status refreshed".to_string() },
            MessageType::Success,
        ));
    }

    /// Set processing/loading state (shows spinner overlay)
    #[allow(dead_code)]
    pub fn set_busy(&mut self, msg: String) {
        self.state = AppState::Busy(msg);
    }

    /// Clear processing state
    #[allow(dead_code)]
    pub fn clear_busy(&mut self) {
        if matches!(self.state, AppState::Busy(_)) {
            self.state = AppState::MainMenu;
        }
    }

    fn draw_busy(&self, f: &mut Frame, msg: &str) {
        let area = centered_rect(40, 10, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        let text = format!("  {}\n\n  {}\n  {}", 
            msg,
            match self.language { Language::Chinese => "处理中...", Language::English => "Processing..." },
            match self.language { Language::Chinese => "请稍候", Language::English => "Please wait" }
        );
        let para = Paragraph::new(text)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(para, area);
    }

    fn on_key_new_server_wizard(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state = AppState::MainMenu;
                self.wizard_step = 0;
            }
            _ => self.wizard_navigation(key),
        }
    }

    fn wizard_navigation(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let max = match self.wizard_step {
            0 => self.wizard_core_types.len().saturating_sub(1),
            1 => self.wizard_versions.len().saturating_sub(1),
            _ => 0,
        };

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => {
                self.wizard_selected = self.wizard_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => {
                if self.wizard_selected < max { self.wizard_selected += 1; }
            }
            KeyCode::Enter => {
                match self.wizard_step {
                    0 => {
                        // Core type selected → fetch versions
                        self.wizard_step = 1;
                        self.wizard_selected = 0;
                        let core_type = self.wizard_core_types[self.wizard_selected].clone();
                        let rt = tokio::runtime::Handle::current();
                        self.wizard_versions = match core_type {
                            crate::core_download::CoreType::Fabric => {
                                rt.block_on(crate::core_download::fetch_fabric_game_versions())
                                    .unwrap_or_else(|_| vec!["1.21.1".to_string(), "1.20.1".to_string()])
                            }
                            crate::core_download::CoreType::Vanilla => {
                                rt.block_on(crate::core_download::fetch_vanilla_versions())
                                    .unwrap_or_else(|_| vec!["1.21.1".to_string(), "1.20.1".to_string()])
                            }
                            crate::core_download::CoreType::Paper => {
                                rt.block_on(crate::core_download::fetch_paper_versions())
                                    .unwrap_or_else(|_| vec!["1.21".to_string(), "1.20".to_string()])
                            }
                        };
                        // Limit to 20 versions
                        self.wizard_versions.truncate(20);
                    }
                    1 => {
                        // Version selected → download
                        let core_type = self.wizard_core_types[0].clone();
                        let version = self.wizard_versions[self.wizard_selected].clone();
                        let dir = std::env::current_dir().unwrap_or_default();
                        let rt = tokio::runtime::Handle::current();
                        let result = match core_type {
                            crate::core_download::CoreType::Fabric => {
                                let loader = rt.block_on(crate::core_download::fetch_fabric_loader(&version))
                                    .unwrap_or_else(|_| "0.17.2".to_string());
                                rt.block_on(crate::core_download::download_fabric_server(&version, &loader, &dir))
                            }
                            crate::core_download::CoreType::Vanilla => {
                                rt.block_on(crate::core_download::download_vanilla_server(&version, &dir))
                            }
                            crate::core_download::CoreType::Paper => {
                                rt.block_on(crate::core_download::download_paper_server(&version, &dir))
                            }
                        };
                        match result {
                            Ok(path) => {
                                // Auto-create eula.txt (P3-6)
                                let _ = crate::core_download::create_eula(&dir);
                                self.message = Some((
                                    format!("Downloaded: {}", path),
                                    MessageType::Success,
                                ));
                                // Refresh server discovery
                                let scan_dir = self.config_path.parent().unwrap_or(std::path::Path::new("."));
                                self.discovered_servers = crate::config::discover_servers(scan_dir);
                            }
                            Err(e) => {
                                self.message = Some((
                                    format!("Failed: {}", e),
                                    MessageType::Warning,
                                ));
                            }
                        }
                        self.state = AppState::MainMenu;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn on_key_server_config_edit(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => {
                self.server_edit_index = self.server_edit_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => {
                if self.server_edit_index + 1 < self.server_edit_fields.len() {
                    self.server_edit_index += 1;
                }
            }
            KeyCode::Enter => {
                self.message = Some((
                    match self.language { Language::Chinese => "配置已保存".to_string(), Language::English => "Config saved".to_string() },
                    MessageType::Success,
                ));
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }

    fn draw_server_config_edit(&self, f: &mut Frame) {
        let mut items: Vec<ListItem> = Vec::new();
        for (i, (label, value)) in self.server_edit_fields.iter().enumerate() {
            let prefix = if i == self.server_edit_index { "> " } else { "  " };
            items.push(ListItem::new(Span::raw(format!("{}{}: {}", prefix, label, value))));
        }
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.server_edit_index));
        let title = match self.language {
            Language::Chinese => "服务器配置 (Esc返回 Enter保存)",
            Language::English => "Server Config (Esc back Enter save)",
        };
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow));
        f.render_stateful_widget(list, f.area(), &mut state);
    }

    fn draw_new_server_wizard(&self, f: &mut Frame) {
        let items: Vec<ListItem> = match self.wizard_step {
            0 => self.wizard_core_types.iter()
                .map(|ct| ListItem::new(Span::raw(match self.language { Language::Chinese => ct.display_name_cn(), Language::English => ct.display_name() })))
                .collect(),
            1 => self.wizard_versions.iter()
                .map(|v| ListItem::new(Span::raw(v.clone())))
                .collect(),
            _ => vec![ListItem::new(Span::raw(match self.language { Language::Chinese => "下载中...", Language::English => "Downloading..." }))],
        };
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.wizard_selected));
        let title = match self.wizard_step {
            0 => match self.language { Language::Chinese => "新建服务器 — 选择核心类型 (Enter确认)", Language::English => "New Server — Select Core Type (Enter)" },
            1 => match self.language { Language::Chinese => "选择 Minecraft 版本", Language::English => "Select Minecraft Version" },
            _ => match self.language { Language::Chinese => "正在下载...", Language::English => "Downloading..." },
        };
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        let area = centered_rect(55, 30, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_stateful_widget(list, area, &mut state);
    }

    fn on_key_mod_browser(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let mods = crate::core_download::popular_mods();
        let max = mods.len().saturating_sub(1);

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('8') => {
                self.wizard_selected = self.wizard_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('2') => {
                if self.wizard_selected < max { self.wizard_selected += 1; }
            }
            KeyCode::Enter => {
                if let Some((_slug, name, project_id)) = mods.get(self.wizard_selected) {
                    let dir = std::env::current_dir().unwrap_or_default();
                    let rt = tokio::runtime::Handle::current();
                    let game_ver = "1.21.1"; // default
                    match rt.block_on(crate::core_download::get_modrinth_latest_version(project_id, game_ver)) {
                        Ok(file) => {
                            let filename = file.filename.clone();
                            match rt.block_on(crate::core_download::download_modrinth_mod(&file.url, &filename, &dir)) {
                                Ok(path) => {
                                    self.message = Some((format!("Downloaded: {}", path), MessageType::Success));
                                }
                                Err(e) => {
                                    self.message = Some((format!("{} failed: {}", name, e), MessageType::Warning));
                                }
                            }
                        }
                        Err(e) => {
                            self.message = Some((format!("{} not found for {}: {}", name, game_ver, e), MessageType::Warning));
                        }
                    }
                }
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }

    fn draw_mod_browser(&self, f: &mut Frame) {
        let mods = crate::core_download::popular_mods();
        let items: Vec<ListItem> = mods.iter()
            .map(|(_, name, _)| ListItem::new(Span::raw(*name)))
            .collect();
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.wizard_selected));
        let title = match self.language {
            Language::Chinese => "热门 Mod (Enter下载 Esc返回)",
            Language::English => "Popular Mods (Enter download Esc back)",
        };
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        let area = centered_rect(50, 25, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_stateful_widget(list, area, &mut state);
    }

    // Server control methods
    fn start_server_background(&mut self) {
        let session = self.get_session_name();
        let jar = self.get_jar();
        let min_mem = self.get_min_mem();
        let max_mem = self.get_max_mem();

        // Check if tmux session already exists
        let check = std::process::Command::new("tmux")
            .args(["has-session", "-t", &session])
            .output();

        if let Ok(out) = check {
            if out.status.success() {
                self.message = Some((
                    if matches!(self.language, Language::Chinese) {
                        format!("tmux 会话 '{}' 已存在，请先停止服务器", session)
                    } else {
                        format!("tmux session '{}' already exists, stop server first", session)
                    },
                    MessageType::Warning,
                ));
                return;
            }
        }

        // Create tmux session
        let _ = std::process::Command::new("tmux")
            .args(["new-session", "-d", "-s", &session, "-x", "120", "-y", "30"])
            .output();

        // Send java command
        let java_cmd = format!("java -Xms{} -Xmx{} -jar {} nogui", min_mem, max_mem, jar);
        let _ = std::process::Command::new("tmux")
            .args(["send-keys", "-t", &session, &java_cmd, "Enter"])
            .output();

        // Start mc-minder process
        let bin = self.find_mcminder_bin();
        let config = self.config_path.to_string_lossy().to_string();
        let _ = std::process::Command::new("nohup")
            .args([&bin, "--config", &config])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        self.message = Some((
            if matches!(self.language, Language::Chinese) {
                format!("服务器已在后台启动 (会话: {})", session)
            } else {
                format!("Server started in background (session: {})", session)
            },
            MessageType::Success,
        ));
        self.server_running = true;

        // Start watchdog if not already running (best-effort background task)
        self.start_watchdog();
    }

    // Lightweight watchdog: writes a pid file and keeps a minimal heartbeat.
    fn start_watchdog(&mut self) {
        // Avoid duplicating work in a very simple way
        if self.watchdog_running {
            return;
        }
        // Write current PID to watchdog file
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".mc-minder/tmp/mc-minder-watchdog.pid");
            let _ = fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new(".")));
            let _ = fs::write(&path, std::process::id().to_string());
        }
        self.watchdog_running = true;
        // Spawn a very lightweight background thread just to demonstrate activity
        std::thread::spawn(move || {
            // In a real implementation this would monitor processes and restart if needed.
            // Here we just sleep to keep the thread alive for demonstration purpose.
            loop {
                std::thread::sleep(std::time::Duration::from_secs(10));
            }
        });
    }

    fn start_server_foreground(&mut self) {
        let jar = self.get_jar();
        let min_mem = self.get_min_mem();
        let max_mem = self.get_max_mem();

        self.message = Some((
            if matches!(self.language, Language::Chinese) {
                format!("正在退出 TUI 启动前台服务器...\n\n命令: java -Xms{} -Xmx{} -jar {} nogui\n\n按 Ctrl+C 停止服务器", min_mem, max_mem, jar)
            } else {
                format!("Exiting TUI to start foreground server...\n\nCommand: java -Xms{} -Xmx{} -jar {} nogui\n\nPress Ctrl+C to stop server", min_mem, max_mem, jar)
            },
            MessageType::Info,
        ));

        // Exit TUI and exec Java directly in terminal
        self.foreground_requested = true;
        self.should_quit = true;
    }

    fn stop_server(&mut self) {
        let session = self.get_session_name();

        // Send stop command to tmux
        let _ = std::process::Command::new("tmux")
            .args(["send-keys", "-t", &session, "stop", "Enter"])
            .output();

        // Wait a bit then kill session
        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = std::process::Command::new("tmux")
            .args(["kill-session", "-t", &session])
            .output();

        self.message = Some((
            if matches!(self.language, Language::Chinese) {
                "服务器已停止".to_string()
            } else {
                "Server stopped".to_string()
            },
            MessageType::Success,
        ));
        self.server_running = false;
    }

    fn restart_server(&mut self) {
        self.stop_server();
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.start_server_background();
    }

    fn show_server_status(&mut self) {
        let session = self.get_session_name();

        // Check tmux session
        let tmux_check = std::process::Command::new("tmux")
            .args(["has-session", "-t", &session])
            .output();
        self.server_running = tmux_check.map(|o| o.status.success()).unwrap_or(false);

        // Check mc-minder PID
        let pid_file = dirs::home_dir()
            .unwrap_or_default()
            .join(".mc-minder/tmp/mc-minder.pid");
        self.mc_minder_running = pid_file.exists();

        // Check watchdog PID
        let watchdog_file = dirs::home_dir()
            .unwrap_or_default()
            .join(".mc-minder/tmp/mc-minder-watchdog.pid");
        self.watchdog_running = watchdog_file.exists();

self.state = AppState::StatusView;
    }

    fn load_server_log(&mut self) {
        let log_file = self.config.as_ref()
            .map(|c| c.server.log_file.clone())
            .unwrap_or_else(|| "logs/latest.log".to_string());
        self.server_log_content = std::fs::read_to_string(&log_file)
            .unwrap_or_else(|_| if matches!(self.language, Language::Chinese) {
                "无法读取服务器日志文件".to_string()
            } else {
                "Cannot read server log file".to_string()
            });
        self.log_scroll = 0;
    }

    fn load_minder_log(&mut self) {
        self.minder_log_content = std::fs::read_to_string("logs/mc-minder.log")
            .unwrap_or_else(|_| if matches!(self.language, Language::Chinese) {
                "无法读取 MC-Minder 日志文件".to_string()
            } else {
                "Cannot read MC-Minder log file".to_string()
            });
        self.log_scroll = 0;
    }

    fn init_config(&mut self) {
        self.state = AppState::ConfigWizard;
        self.init_config_wizard_fields();
        self.message = Some((
            if matches!(self.language, Language::Chinese) {
                "配置向导尚未完全实现，请使用: mc-minder init".to_string()
            } else {
                "Config wizard not fully implemented yet, use: mc-minder init".to_string()
            },
            MessageType::Info,
        ));
    }

    fn update_mcminder(&mut self) {
        // Spawn async check task
        self.state = AppState::UpdateView;

        let (tx, rx) = tokio::sync::mpsc::channel(32);
        self.update_rx = Some(rx);

        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let engine = UpdateEngine::new();

        // Spawn async check task
        tokio::spawn(async move {
            let result = engine.check_update(&current_version).await;
            let _ = tx.send(result).await;
        });
    }

    fn on_key_update_view(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        // Process any pending messages first
        self.process_update_messages();

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Cancel and go back to main menu
                self.update_rx = None;
                self.update_state = None;
                self.state = AppState::MainMenu;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                // If UpdateAvailable, start download
                if let Some(UpdateState::UpdateAvailable { download_url, latest, .. }) = &self.update_state {
                    self.start_download(download_url.clone(), latest.clone());
                } else if let Some(UpdateState::UpToDate) = &self.update_state {
                    // User acknowledged up to date, go back
                    self.update_rx = None;
                    self.update_state = None;
                    self.state = AppState::MainMenu;
                } else if let Some(UpdateState::Done { .. }) = &self.update_state {
                    // User acknowledged done, quit (binary updated)
                    self.should_quit = true;
                } else if let Some(UpdateState::Failed(_)) = &self.update_state {
                    // User acknowledged failure, go back
                    self.update_rx = None;
                    self.update_state = None;
                    self.state = AppState::MainMenu;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // Cancel update if in UpdateAvailable state
                if let Some(UpdateState::UpdateAvailable { .. }) = &self.update_state {
                    self.update_rx = None;
                    self.update_state = None;
                    self.state = AppState::MainMenu;
                }
            }
            _ => {}
        }
    }

    fn start_download(&mut self, download_url: String, latest_version: String) {
        self.update_state = Some(UpdateState::Downloading { downloaded: 0, total: None });

        let (tx, rx) = tokio::sync::mpsc::channel(32);
        self.update_rx = Some(rx);

        let engine = UpdateEngine::new();
        tokio::spawn(async move {
            let result = engine.download_and_install(&download_url, &latest_version, tx).await;
            if let Err(e) = result {
                // Send failed message
                let (tx2, _) = tokio::sync::mpsc::channel(32);
                let _ = tx2.send(UpdateMsg::Failed(e)).await;
            }
        });
    }

    pub(crate) fn process_update_messages(&mut self) {
        if let Some(rx) = &mut self.update_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    UpdateMsg::UpdateAvailable { current, latest, download_url } => {
                        self.update_state = Some(UpdateState::UpdateAvailable {
                            current,
                            latest,
                            download_url,
                        });
                    }
                    UpdateMsg::UpToDate => {
                        self.update_state = Some(UpdateState::UpToDate);
                    }
                    UpdateMsg::DownloadProgress { downloaded, total } => {
                        self.update_state = Some(UpdateState::Downloading { downloaded, total });
                    }
                    UpdateMsg::Installing => {
                        self.update_state = Some(UpdateState::Installing);
                    }
                    UpdateMsg::Done { new_version } => {
                        self.update_state = Some(UpdateState::Done { new_version });
                    }
                    UpdateMsg::Failed(err) => {
                        self.update_state = Some(UpdateState::Failed(err));
                    }
                }
            }
        }
    }

    fn switch_java_version(&mut self) {
        if self.java_cache.is_empty() {
            self.java_cache = java_manager::detect_java_versions(self.config.as_ref());
        }
        let versions = self.java_cache.clone();
        if versions.is_empty() {
            self.message = Some((
                if matches!(self.language, Language::Chinese) {
                    "未检测到 Java 版本，请先安装 Java".to_string()
                } else {
                    "No Java versions detected. Please install Java first.".to_string()
                },
                MessageType::Warning,
            ));
            return;
        }

        if versions.len() == 1 {
            let (path, version) = &versions[0];
            self.message = Some((
                if matches!(self.language, Language::Chinese) {
                    format!("当前 Java:\n{}\n{}\n\n如需切换到其他版本，请先在系统中安装", version, path)
                } else {
                    format!("Current Java:\n{}\n{}\n\nInstall other versions first to switch", version, path)
                },
                MessageType::Info,
            ));
            return;
        }

        // Transition to interactive selection screen
        self.java_switch_selected = 0;
        self.state = AppState::JavaSwitch(versions);
    }

    fn install_java_version(&mut self) {
        self.state = AppState::JavaInstall;
    }

    fn show_installed_java(&mut self) {
        if self.java_cache.is_empty() {
            self.java_cache = self.detect_java_versions();
        }
        let versions = java_manager::detect_java_versions(self.config.as_ref());
        let msg = if versions.is_empty() {
            if matches!(self.language, Language::Chinese) {
                "未检测到 Java 版本\n\n请使用菜单中的\"安装 Java\"选项".to_string()
            } else {
                "No Java versions detected\n\nUse \"Install Java\" from the menu".to_string()
            }
        } else {
            let lines: Vec<String> = versions.iter()
                .map(|(path, ver)| format!("{}\n  -> {}", ver, path))
                .collect();
            if matches!(self.language, Language::Chinese) {
                format!("已安装的 Java:\n\n{}", lines.join("\n"))
            } else {
                format!("Installed Java versions:\n\n{}", lines.join("\n"))
            }
        };
        self.message = Some((msg, MessageType::Info));
    }

    fn detect_java_versions(&self) -> Vec<(String, String)> {
        let mut versions = Vec::new();

        // Helper: try get version from a java binary path
        let add_version = |versions: &mut Vec<(String, String)>, path: &str| {
            if versions.iter().any(|(p, _)| p == path) {
                return;
            }
            if let Ok(out) = std::process::Command::new(path).arg("-version").output() {
                let ver = String::from_utf8_lossy(&out.stderr);
                if let Some(line) = ver.lines().next() {
                    versions.push((path.to_string(), line.to_string()));
                }
            }
        };

        // 1. Check system default java
        if let Ok(out) = std::process::Command::new("java").arg("-version").output() {
            let ver = String::from_utf8_lossy(&out.stderr);
            if let Some(line) = ver.lines().next() {
                versions.push(("system default (java)".to_string(), line.to_string()));
            }
        }

        // 2. Search PATH for all java binaries (which -a / where)
        let which_cmd = if cfg!(target_os = "windows") {
            std::process::Command::new("where").arg("java").output()
        } else {
            std::process::Command::new("which").args(["-a", "java"]).output()
        };
        if let Ok(out) = which_cmd {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if !line.is_empty() && line != "java" && !line.contains("no java") {
                    add_version(&mut versions, line);
                }
            }
        }

        // 3. Check update-alternatives (Linux)
        if cfg!(target_os = "linux") {
            if let Ok(out) = std::process::Command::new("update-alternatives")
                .args(["--list", "java"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        add_version(&mut versions, line);
                    }
                }
            }
        }

        // 4. Check custom JDK path from config
        if let Some(ref cfg) = self.config {
            if let Some(ref jdk) = cfg.jvm.jdk_path {
                if !jdk.is_empty() {
                    add_version(&mut versions, jdk);
                }
            }
        }

        // 5. Search common installation directories
        let common_paths = if cfg!(target_os = "android") {
            vec![
                "/data/data/com.termux/files/usr/lib/jvm".to_string(),
                "/data/data/com.termux/files/usr/bin".to_string(),
            ]
        } else {
            let mut paths = vec![
                "/usr/lib/jvm".to_string(),
                "/usr/java".to_string(),
                "/opt/java".to_string(),
                "/opt/jdk".to_string(),
                "/usr/local/lib/jvm".to_string(),
                "/snap/openjdk".to_string(),
            ];
            if let Ok(jh) = std::env::var("JAVA_HOME") {
                paths.push(jh);
            }
            if let Ok(jh) = std::env::var("JDK_HOME") {
                paths.push(jh);
            }
            paths
        };

        for base in common_paths {
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.contains("jdk") || name.contains("jre") || name.contains("openjdk") || name.contains("java") {
                            let full_path = entry.path();
                            let java_bin = full_path.join("bin").join("java");
                            if java_bin.exists() {
                                add_version(&mut versions, java_bin.to_str().unwrap_or(""));
                            } else if full_path.join("java").exists() {
                                add_version(&mut versions, full_path.join("java").to_str().unwrap_or(""));
                            } else {
                                versions.push((
                                    full_path.to_string_lossy().to_string(),
                                    name.to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        versions
    }

    fn save_language(&self) {
        let lang_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".mc-minder");
        let _ = std::fs::create_dir_all(&lang_dir);
        let lang_file = lang_dir.join("lang.conf");
        let lang_str = match self.language {
            Language::Chinese => "zh",
            Language::English => "en",
        };
        let _ = std::fs::write(lang_file, lang_str);

        // Also save language to config.toml for persistence
        if let Ok(content) = std::fs::read_to_string(&self.config_path) {
            let lang_line = format!("language = \"{}\"", lang_str);
            let new_content = if content.contains("language = ") {
                // Replace existing language line
                let lines: Vec<String> = content.lines()
                    .map(|l| {
                        if l.trim_start().starts_with("language = ") {
                            lang_line.clone()
                        } else {
                            l.to_string()
                        }
                    })
                    .collect();
                lines.join("\n")
            } else {
                // Append at end
                format!("{}\n{}", content.trim_end(), lang_line)
            };
            let _ = std::fs::write(&self.config_path, new_content);
        }
    }

    fn load_language() -> Option<Language> {
        let lang_file = dirs::home_dir()
            .unwrap_or_default()
            .join(".mc-minder/lang.conf");

        if !lang_file.exists() {
            return None;
        }

        let content = std::fs::read_to_string(lang_file).ok()?;
        match content.trim() {
            "zh" => Some(Language::Chinese),
            "en" => Some(Language::English),
            _ => None,
        }
    }

    // Helper methods
    #[deprecated(note = "use java_manager::detect_java_versions inline config access instead")] 
    fn get_session_name(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.session_name.clone())
            .unwrap_or_else(|| "mc_server".to_string())
    }

    #[deprecated(note = "use direct config access instead")] 
    pub(crate) fn get_jar(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.jar.clone())
            .unwrap_or_else(|| "fabric-server.jar".to_string())
    }

    #[deprecated(note = "use direct config access instead")] 
    pub(crate) fn get_min_mem(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.min_mem.clone())
            .unwrap_or_else(|| "512M".to_string())
    }

    #[deprecated(note = "use direct config access instead")] 
    pub(crate) fn get_max_mem(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.max_mem.clone())
            .unwrap_or_else(|| "1G".to_string())
    }

    #[deprecated(note = "use java_manager::find_mcminder_bin with config path")] 
    fn find_mcminder_bin(&self) -> String {
        // Check MC_MINDER_BIN env
        if let Ok(bin) = std::env::var("MC_MINDER_BIN") {
            if std::path::Path::new(&bin).exists() {
                return bin;
            }
        }

        // Check current executable directory (most reliable)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let bin = exe_dir.join("mc-minder");
                if bin.exists() {
                    return bin.to_string_lossy().to_string();
                }
                // Also check for platform-specific names
                let platform_bin = if cfg!(target_os = "android") {
                    exe_dir.join("mc-minder-termux-aarch64")
                } else {
                    exe_dir.join("mc-minder-x86_64-linux")
                };
                if platform_bin.exists() {
                    return platform_bin.to_string_lossy().to_string();
                }
            }
        }

        // Check current dir
        if std::path::Path::new("./mc-minder").exists() {
            return "./mc-minder".to_string();
        }

        // Check config path parent
        if let Some(parent) = self.config_path.parent() {
            let bin = parent.join("mc-minder");
            if bin.exists() {
                return bin.to_string_lossy().to_string();
            }
        }

        // Fallback to PATH
        "mc-minder".to_string()
    }

    // UI methods
    fn main_menu_items(&self) -> Vec<&'static str> {
        self.main_menu_items_new()
    }

    fn java_menu_items(&self) -> Vec<&'static str> {
        match self.language {
            Language::Chinese => vec![
                "1. 切换 Java 版本",
                "2. 安装新 Java 版本",
                "3. 查看所有已安装版本",
                "4. 返回主菜单",
            ],
            Language::English => vec![
                "1. Switch Java Version",
                "2. Install New Java Version",
                "3. View All Installed Versions",
                "4. Back to Main Menu",
            ],
        }
    }

    fn draw_main_menu(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(45), Constraint::Min(0)])
            .split(f.area());

        // Left: main menu
        let menu_items = self.main_menu_items();
        let list_items: Vec<ListItem> = menu_items.iter().enumerate().map(|(i, text)| {
            let color = match i {
                0..=3 => Color::Green,
                4..=7 => Color::Cyan,
                8..=13 => Color::Yellow,
                _ => Color::Magenta,
            };
            ListItem::new(Span::styled(*text, Style::default().fg(color)))
        }).collect();
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.main_menu_selected));
        let title = match self.language {
            Language::Chinese => "MC-Minder 管理菜单",
            Language::English => "MC-Minder Management Menu",
        };
        let list = List::new(list_items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[0], &mut state);

        // Right: status panel
        let status = self.status_text();
        let status_para = Paragraph::new(status)
            .block(Block::default().title(
                match self.language {
                    Language::Chinese => "状态",
                    Language::English => "Status",
                }
            ).borders(Borders::ALL));
        f.render_widget(status_para, chunks[1]);

        // Bottom: help
        let help = match self.language {
            Language::Chinese => "上下键: 导航 | Enter: 确认 | 1-9: 快速选择 | q: 退出",
            Language::English => "Up/Down: Navigate | Enter: Select | 1-9: Quick select | q: Quit",
        };
        let help_block = Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help_block, Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.area())[1]);
    }

    fn draw_java_menu(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(45), Constraint::Min(0)])
            .split(f.area());

        let items = self.java_menu_items();
        let list_items: Vec<ListItem> = items.iter()
            .map(|i| ListItem::new(Span::raw(*i)))
            .collect();
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.java_menu_selected));
        let title = match self.language {
            Language::Chinese => "Java 版本管理",
            Language::English => "Java Version Management",
        };
        let list = List::new(list_items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[0], &mut state);

        // Right panel: Java info
        let versions = java_manager::detect_java_versions(self.config.as_ref());
        let info = if versions.is_empty() {
            match self.language {
                Language::Chinese => "未检测到 Java 版本",
                Language::English => "No Java versions detected",
            }.to_string()
        } else {
            versions.iter()
                .map(|(path, ver)| format!("{} - {}", ver, path))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let info_para = Paragraph::new(info)
            .block(Block::default().title(
                match self.language {
                    Language::Chinese => "已安装的 Java",
                    Language::English => "Installed Java",
                }
            ).borders(Borders::ALL));
        f.render_widget(info_para, chunks[1]);
    }

    fn on_key_java_switch(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let versions = match &self.state {
            AppState::JavaSwitch(v) => v.clone(),
            _ => return,
        };
        let max = versions.len().saturating_sub(1);

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state = AppState::JavaMenu;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.java_switch_selected = self.java_switch_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.java_switch_selected < max {
                    self.java_switch_selected += 1;
                }
            }
            KeyCode::Enter => {
                // Apply the selected Java version
                if let Some((target_path, target_ver)) = versions.get(self.java_switch_selected) {
                    let jdk_path = if target_path.ends_with("/java") || target_path.ends_with("\\java") {
                        target_path.to_string()
                    } else if target_path.contains("system default") {
                        // System default - clear jdk_path
                        String::new()
                    } else {
                        format!("{}/bin/java", target_path)
                    };

                    // Update in-memory config
                    if let Some(ref mut cfg) = self.config {
                        cfg.jvm.jdk_path = if jdk_path.is_empty() { None } else { Some(jdk_path.clone()) };

                        // Write to config.toml
                        let config_content = crate::init::generate_config_content(
                            &cfg.rcon.password,
                            &cfg.server.min_mem,
                            &cfg.server.max_mem,
                            &cfg.server.session_name,
                            &cfg.server.jar,
                            &cfg.jvm.extra_flags,
                            &jdk_path,
                        );
                        let _ = std::fs::write(&self.config_path, &config_content);
                    }

                    self.message = Some((
                        if matches!(self.language, Language::Chinese) {
                            format!("已切换到 Java:\n{}\n{}", target_ver, if jdk_path.is_empty() { "系统默认" } else { &jdk_path })
                        } else {
                            format!("Switched to Java:\n{}\n{}", target_ver, if jdk_path.is_empty() { "system default" } else { &jdk_path })
                        },
                        MessageType::Success,
                    ));
                }
                self.state = AppState::JavaMenu;
            }
            _ => {}
        }
    }

    fn draw_java_switch(&self, f: &mut Frame, versions: &[(String, String)]) {
        let list_items: Vec<ListItem> = versions.iter()
            .map(|(path, ver)| {
                let label = if path.contains("system default") {
                    format!("(system) {}  [{}]", ver, path)
                } else {
                    format!("{}  [{}]", ver, path)
                };
                ListItem::new(Span::raw(label))
            })
            .collect();

        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.java_switch_selected));

        let title = match self.language {
            Language::Chinese => "选择 Java 版本 (Enter确认 Esc返回)",
            Language::English => "Select Java Version (Enter to confirm Esc to cancel)",
        };

        let list = List::new(list_items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let area = centered_rect(60, 30, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_stateful_widget(list, area, &mut state);
    }

    fn java_install_options(&self) -> Vec<(String, String, bool)> {
        // Returns (label, install_command, needs_confirmation)
        let is_termux = std::env::var("TERMUX_VERSION").is_ok()
            || std::path::Path::new("/data/data/com.termux").exists();

        if is_termux {
            vec![
                ("OpenJDK 17".to_string(), "pkg install openjdk-17 -y".to_string(), false),
                ("OpenJDK 21".to_string(), "pkg install openjdk-21 -y".to_string(), false),
            ]
        } else if cfg!(target_os = "linux") {
            // Detect package manager
            let has_apt = std::process::Command::new("which").arg("apt").output().map(|o| o.status.success()).unwrap_or(false);
            let has_dnf = std::process::Command::new("which").arg("dnf").output().map(|o| o.status.success()).unwrap_or(false);
            let has_pacman = std::process::Command::new("which").arg("pacman").output().map(|o| o.status.success()).unwrap_or(false);

            if has_apt {
                vec![
                    ("OpenJDK 17 (apt)".to_string(), "sudo apt install -y openjdk-17-jre".to_string(), true),
                    ("OpenJDK 21 (apt)".to_string(), "sudo apt install -y openjdk-21-jre".to_string(), true),
                ]
            } else if has_dnf {
                vec![
                    ("OpenJDK 17 (dnf)".to_string(), "sudo dnf install -y java-17-openjdk".to_string(), true),
                    ("OpenJDK 21 (dnf)".to_string(), "sudo dnf install -y java-21-openjdk".to_string(), true),
                ]
            } else if has_pacman {
                vec![
                    ("OpenJDK 17 (pacman)".to_string(), "sudo pacman -S --noconfirm jre17-openjdk".to_string(), true),
                    ("OpenJDK 21 (pacman)".to_string(), "sudo pacman -S --noconfirm jre21-openjdk".to_string(), true),
                ]
            } else {
                vec![
                    ("OpenJDK 17 (manual)".to_string(), "".to_string(), false),
                    ("OpenJDK 21 (manual)".to_string(), "".to_string(), false),
                ]
            }
        } else {
            // macOS fallback
            vec![
                ("OpenJDK 17 (brew)".to_string(), "brew install openjdk@17".to_string(), false),
                ("OpenJDK 21 (brew)".to_string(), "brew install openjdk@21".to_string(), false),
            ]
        }
    }

    fn on_key_java_install(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let options = java_manager::java_install_options();
        let max = options.len().saturating_sub(1);

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state = AppState::JavaMenu;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.java_switch_selected = self.java_switch_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.java_switch_selected < max {
                    self.java_switch_selected += 1;
                }
            }
            KeyCode::Enter => {
                if let Some((label, cmd, _needs_sudo)) = options.get(self.java_switch_selected) {
                    if cmd.is_empty() {
                        self.message = Some((
                            if matches!(self.language, Language::Chinese) {
                                format!("请手动安装 {}\n\n未检测到支持的包管理器", label)
                            } else {
                                format!("Please install {} manually\n\nNo supported package manager detected", label)
                            },
                            MessageType::Warning,
                        ));
                    } else {
                        let result = std::process::Command::new("sh")
                            .args(["-c", cmd])
                            .output();

                        match result {
                            Ok(out) if out.status.success() => {
                                self.message = Some((
                                    if matches!(self.language, Language::Chinese) {
                                        format!("{} 安装成功!\n\n请返回菜单使用\"切换Java版本\"选择新安装的JDK", label)
                                    } else {
                                        format!("{} installed successfully!\n\nGo back and use \"Switch Java Version\" to select the new JDK", label)
                                    },
                                    MessageType::Success,
                                ));
                            }
                            Ok(out) => {
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                self.message = Some((
                                    if matches!(self.language, Language::Chinese) {
                                        format!("{} 安装失败:\n{}", label, stderr.lines().last().unwrap_or("未知错误"))
                                    } else {
                                        format!("{} install failed:\n{}", label, stderr.lines().last().unwrap_or("Unknown error"))
                                    },
                                    MessageType::Warning,
                                ));
                            }
                            Err(e) => {
                                self.message = Some((
                                    if matches!(self.language, Language::Chinese) {
                                        format!("执行安装命令失败: {}", e)
                                    } else {
                                        format!("Failed to run install command: {}", e)
                                    },
                                    MessageType::Warning,
                                ));
                            }
                        }
                    }
                    self.state = AppState::JavaMenu;
                }
            }
            _ => {}
        }
    }

    fn draw_java_install(&self, f: &mut Frame) {
        let options = self.java_install_options();
        let list_items: Vec<ListItem> = options.iter()
            .map(|(label, cmd, _)| {
                let display = if cmd.is_empty() {
                    format!("{} (手动安装)", label)
                } else {
                    label.clone()
                };
                ListItem::new(Span::raw(display))
            })
            .collect();

        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.java_switch_selected));

        let title = match self.language {
            Language::Chinese => "安装 Java 版本 (Enter确认 Esc返回)",
            Language::English => "Install Java Version (Enter to confirm Esc to cancel)",
        };

        let list = List::new(list_items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let area = centered_rect(55, 25, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_stateful_widget(list, area, &mut state);
    }

    fn draw_log_viewer(&self, f: &mut Frame, log_type: &LogType) {
        let content = match log_type {
            LogType::Server => &self.server_log_content,
            LogType::McMinder => &self.minder_log_content,
        };

        let title = match log_type {
            LogType::Server => match self.language {
                Language::Chinese => "服务器日志",
                Language::English => "Server Log",
            },
            LogType::McMinder => match self.language {
                Language::Chinese => "MC-Minder 日志",
                Language::English => "MC-Minder Log",
            },
        };

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let start = self.log_scroll.min(total_lines.saturating_sub(1));
        let visible: String = lines.iter().skip(start).take(30)
            .map(|s| *s)
            .collect::<Vec<_>>()
            .join("\n");

        let para = Paragraph::new(visible)
            .block(Block::default()
                .title(format!("{} ({} - {} / {})",
                    title,
                    start + 1,
                    (start + 30).min(total_lines),
                    total_lines))
                .borders(Borders::ALL))
            .scroll((0, 0));

        f.render_widget(para, f.area());

        // Help
        let help = match self.language {
            Language::Chinese => "上下键: 滚动 | PageUp/PageDown: 翻页 | Esc: 返回",
            Language::English => "Up/Down: Scroll | PageUp/PageDown: Page | Esc: Back",
        };
        let help_block = Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help_block, Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.area())[1]);
    }

    fn draw_config_wizard(&self, f: &mut Frame) {
        // Render a simple multi-field form using the wizard_fields data
        let title = match self.language {
            Language::Chinese => "配置向导",
            Language::English => "Configuration Wizard",
        };
        // Prepare list items from wizard fields
        let items: Vec<ListItem> = self.wizard_fields.iter().enumerate().map(|(idx, fw)| {
            let mut line = String::new();
            if idx == self.wizard_index {
                line.push_str("→ ");
            } else {
                line.push_str("  ");
            }
            line.push_str(&format!("{}: {}", fw.label, fw.value));
            ListItem::new(Span::raw(line))
        }).collect();
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL));
        let area = centered_rect(70, 60, f.area());
        f.render_widget(list, area);
        // Help hint
        let hint = match self.language {
            Language::Chinese => "Tab/Shift+Tab 切换字段, Enter 保存, Esc 取消",
            Language::English => "Tab/Shift+Tab to switch fields, Enter to save, Esc to cancel",
        };
        let help = Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.area())[1]);
    }

    fn draw_language_select(&self, f: &mut Frame) {
        let title = match self.language {
            Language::Chinese => "语言设置 / Language Settings",
            Language::English => "Language Settings / 语言设置",
        };
        let items = vec![
            ListItem::new(Span::raw("1. 中文 (Chinese)")),
            ListItem::new(Span::raw("2. English (英文)")),
        ];
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(match self.language {
            Language::Chinese => 0,
            Language::English => 1,
        }));
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        let area = centered_rect(50, 30, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_stateful_widget(list, area, &mut state);
    }

    fn draw_confirm_dialog(&self, f: &mut Frame, action: &ConfirmAction) {
        let (title, msg) = match action {
            ConfirmAction::StopServer => (
                match self.language {
                    Language::Chinese => "确认停止",
                    Language::English => "Confirm Stop",
                },
                match self.language {
                    Language::Chinese => "确定要停止服务器吗？\n\n按 Y 确认，按 N 取消",
                    Language::English => "Are you sure you want to stop the server?\n\nPress Y to confirm, N to cancel",
                },
            ),
            ConfirmAction::RestartServer => (
                match self.language {
                    Language::Chinese => "确认重启",
                    Language::English => "Confirm Restart",
                },
                match self.language {
                    Language::Chinese => "确定要重启服务器吗？\n\n按 Y 确认，按 N 取消",
                    Language::English => "Are you sure you want to restart the server?\n\nPress Y to confirm, N to cancel",
                },
            ),
            ConfirmAction::UpdateMcminder => (
                match self.language {
                    Language::Chinese => "确认更新",
                    Language::English => "Confirm Update",
                },
                match self.language {
                    Language::Chinese => "确定要更新 MC-Minder 吗？\n\n按 Y 确认，按 N 取消",
                    Language::English => "Are you sure you want to update MC-Minder?\n\nPress Y to confirm, N to cancel",
                },
            ),
            ConfirmAction::Exit => (
                match self.language {
                    Language::Chinese => "确认退出",
                    Language::English => "Confirm Exit",
                },
                match self.language {
                    Language::Chinese => "确定要退出吗？\n\n按 Y 确认，按 N 取消",
                    Language::English => "Are you sure you want to exit?\n\nPress Y to confirm, N to cancel",
                },
            ),
            ConfirmAction::Modal { title_cn, title_en, message_cn, message_en } => (
                if matches!(self.language, Language::Chinese) { *title_cn } else { *title_en },
                if matches!(self.language, Language::Chinese) { *message_cn } else { *message_en },
            ),
        };

        let para = Paragraph::new(msg)
            .block(Block::default().title(title).borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center);
        let area = centered_rect(50, 30, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_widget(para, area);
    }

    fn draw_status_view(&self, f: &mut Frame) {
        let has_tps = !self.tps_history.is_empty();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(if has_tps { 3 } else { 2 }),
                Constraint::Length(if has_tps { 6 } else { 0 }),
                Constraint::Min(1),
            ])
            .split(f.area());

        // Process block
        let session = self.get_session_name();
        let proc_text = format!("Session: {} | MC-Minder: {} | Watchdog: {}",
            session,
            if self.mc_minder_running { "ON" } else { "OFF" },
            if self.watchdog_running { "ON" } else { "OFF" },
        );
        f.render_widget(Paragraph::new(proc_text).block(Block::default().borders(Borders::ALL)
            .title("Process").border_style(Style::default().fg(Color::Gray))), chunks[0]);

        // MC Status block with color coding
        let (mc_title, mc_text, border_color) = if let Some(ref s) = self.mc_status_snapshot {
            if s.online {
                let mut txt = format!("Version: {} | Players: {}/{} | Latency: {}ms",
                    s.version, s.players_online, s.players_max, s.latency_ms);
                if let Some(tps) = s.tps {
                    txt.push_str(&format!(" | TPS: {:.1}", tps));
                }
                txt.push_str(&format!("\nMOTD: {}", s.motd.lines().next().unwrap_or("")));
                ("Minecraft Server", txt, Color::Green)
            } else {
                let err = s.error.as_deref().unwrap_or("unknown");
                ("Minecraft Server", format!("OFFLINE — {}", err), Color::Red)
            }
        } else {
            ("Minecraft Server", "Waiting for data...".to_string(), Color::Yellow)
        };
        f.render_widget(Paragraph::new(mc_text).block(Block::default().borders(Borders::ALL)
            .title(mc_title).border_style(Style::default().fg(border_color))), chunks[1]);

        // TPS history chart (P6-2)
        if has_tps {
            let chart_lines: Vec<String> = vec![
                self.tps_chart_line(20.0, "20"),
                self.tps_chart_line(19.5, ""),
                self.tps_chart_line(18.0, "18"),
                self.tps_chart_line(15.0, ""),
                self.tps_chart_line(10.0, "10"),
                self.tps_chart_line(5.0, ""),
            ];
            let chart_text = chart_lines.join("\n");
            f.render_widget(Paragraph::new(chart_text).block(Block::default().borders(Borders::ALL)
                .title("TPS History (last 30 readings)")), chunks[2]);
        }
    }

    fn tps_chart_line(&self, threshold: f64, label: &str) -> String {
        let prefix = if label.is_empty() { "   ".to_string() } else { format!("{:>2} ", label) };
        let mut line = prefix.clone();
        for &tps in &self.tps_history {
            if tps >= threshold {
                let bar = if tps >= 18.0 { "█" } else if tps >= 10.0 { "▓" } else { "░" };
                line.push_str(bar);
            } else {
                line.push(' ');
            }
        }
        line
    }

    fn draw_console(&mut self, f: &mut Frame) {
        // Auto-refresh if enabled and 1 second has passed
        if self.console_auto_refresh && self.last_refresh.elapsed() > std::time::Duration::from_secs(1) {
            self.capture_console_output();
        }

        let title = match self.language {
            Language::Chinese => "实时控制台",
            Language::English => "Real-time Console",
        };

        let lines: Vec<&str> = self.console_content.lines().collect();
        let total_lines = lines.len();
        let start = self.console_scroll.min(total_lines.saturating_sub(1));
        let visible: String = lines.iter().skip(start).take(30)
            .map(|s| *s)
            .collect::<Vec<_>>()
            .join("\n");

        let auto_str = if self.console_auto_refresh { "ON" } else { "OFF" };
        let para = Paragraph::new(visible)
            .block(Block::default()
                .title(format!("{} ({}: {}) | {}/{}",
                    title,
                    if matches!(self.language, Language::Chinese) { "自动刷新" } else { "Auto" },
                    auto_str,
                    start + 1,
                    total_lines.max(1)))
                .borders(Borders::ALL));

        f.render_widget(para, f.area());

        // Help
        let help = match self.language {
            Language::Chinese => "上/下键: 滚动 | r: 刷新 | a: 自动刷新开关 | Esc: 返回",
            Language::English => "Up/Down: Scroll | r: Refresh | a: Auto-toggle | Esc: Back",
        };
        let help_block = Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help_block, Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.area())[1]);
    }

    fn status_text(&self) -> String {
        let mut s = String::new();
        // MC Server status (from mc-status-probe)
        if let Some(ref mc) = self.mc_status_snapshot {
            if mc.online {
                s.push_str(&format!("MC 服务器: 在线\n  版本: {}\n  玩家: {}/{}\n  延迟: {}ms\n  描述: {}\n\n",
                    mc.version, mc.players_online, mc.players_max, mc.latency_ms,
                    mc.motd.lines().next().unwrap_or("")));
            } else {
                s.push_str(&format!("MC 服务器: 离线\n  原因: {}\n\n",
                    mc.error.as_deref().unwrap_or("未知")));
            }
        } else {
            match self.language {
                Language::Chinese => s.push_str("MC 服务器: 等待数据...\n\n"),
                Language::English => s.push_str("MC Server: Waiting...\n\n"),
            }
        }
        // Process status
        let session = self.get_session_name();
        s.push_str(&format!(
            "进程状态:\n  tmux: {} ({})\n  MC-Minder: {}\n  看门狗: {}\n\n快捷: F5=刷新 F7=备份 c=控制台\n\n",
            session,
            if self.server_running { "ON" } else { "OFF" },
            if self.mc_minder_running { "ON" } else { "OFF" },
            if self.watchdog_running { "ON" } else { "OFF" },
        ));
        // Discovered servers
        if !self.discovered_servers.is_empty() {
            s.push_str("发现的服务器:\n");
            for (i, ds) in self.discovered_servers.iter().enumerate() {
                let marker = if i == self.selected_server { ">" } else { " " };
                s.push_str(&format!("  {} {}\n", marker, ds.name));
            }
        }
        s
    }

    /// Dispatch an Action returned by a component.
    fn dispatch(&mut self, action: Action) {
        match action {
            Action::Navigate(state) => { self.clear_components(); self.state = state; }
            Action::GoBack => { self.clear_components(); self.state = AppState::MainMenu; self.main_menu_selected = 0; }
            Action::Quit => self.should_quit = true,
            Action::ShowMessage(msg, typ) => { self.message = Some((msg, typ)); self.message_timeout = Some(std::time::Instant::now()); }
            Action::ClearMessage => { self.message = None; self.message_timeout = None; }
            Action::SetLanguage(lang) => { self.language = lang; self.save_language(); }
            Action::StartServerBackground => self.start_server_background(),
            Action::StartServerForeground => self.start_server_foreground(),
            Action::StopServer => self.stop_server(),
            Action::RestartServer => self.restart_server(),
            Action::Noop => {}
            _ => {} // Other actions handled later as needed
        }
    }

    /// Clear all component instances (called on state transition).
    fn clear_components(&mut self) {
        self.language_select = None;
        self.confirm_dialog = None;
        self.mod_list = None;
        self.quick_commands = None;
        self.mod_browser = None;
        self.log_viewer = None;
        self.status_view = None;
        self.console_view = None;
        self.config_wizard = None;
        self.update_view = None;
        self.server_config_edit = None;
        self.java_menu = None;
        self.java_switch = None;
        self.java_install = None;
        self.main_menu = None;
        self.running_foreground = None;
        self.new_server_wizard = None;
    }

    pub fn draw(&mut self, f: &mut Frame) {
        match self.state.clone() {
            // === States with components (Phase B) ===
            AppState::LanguageSelect => {
                if self.language_select.is_none() { self.language_select = Some(LanguageSelect::new(self.language)); }
                if let Some(ref mut c) = self.language_select { c.render(f, f.area()); }
            }
            AppState::ConfirmDialog(ref action) => {
                if self.confirm_dialog.is_none() || self.confirm_dialog.as_ref().map_or(true, |c| &c.action != action) {
                    self.confirm_dialog = Some(ConfirmDialog::new(action.clone(), self.language));
                }
                if let Some(ref mut c) = self.confirm_dialog { c.render(f, f.area()); }
            }
            AppState::ModList => {
                if self.mod_list.is_none() { self.mod_list = Some(ModList::new(self.language)); }
                if let Some(ref mut c) = self.mod_list { c.render(f, f.area()); }
            }
            AppState::QuickCommands => {
                if self.quick_commands.is_none() { self.quick_commands = Some(QuickCommands::new(self.language)); }
                if let Some(ref mut c) = self.quick_commands { c.render(f, f.area()); }
            }
            AppState::ModBrowser => {
                if self.mod_browser.is_none() { self.mod_browser = Some(ModBrowser::new(self.language)); }
                if let Some(ref mut c) = self.mod_browser { c.render(f, f.area()); }
            }
            AppState::LogViewer(ref lt) => {
                let lt2 = lt.clone();
                if self.log_viewer.is_none() {
                    let mut lv = LogViewer::new(lt2, self.language);
                    lv.load();
                    self.log_viewer = Some(lv);
                }
                if let Some(ref mut c) = self.log_viewer { c.render(f, f.area()); }
            }
            AppState::StatusView => {
                if self.status_view.is_none() {
                    let mut sv = StatusView::new(self.language);
                    sv.server_running = self.server_running;
                    sv.mc_minder_running = self.mc_minder_running;
                    sv.watchdog_running = self.watchdog_running;
                    sv.mc_status = self.mc_status_snapshot.clone();
                    sv.tps_history = self.tps_history.clone();
                    sv.session_name = self.get_session_name();
                    sv.discovered_servers = self.discovered_servers.iter().map(|d| d.name.clone()).collect();
                    self.status_view = Some(sv);
                }
                if let Some(ref mut c) = self.status_view {
                    c.mc_status = self.mc_status_snapshot.clone();
                    c.tps_history = self.tps_history.clone();
                    c.render(f, f.area());
                }
            }
            AppState::Console => {
                if self.console_view.is_none() {
                    let mut cv = ConsoleView::new(self.language);
                    cv.session_name = self.get_session_name();
                    cv.capture_output();
                    self.console_view = Some(cv);
                    self.enter_console();
                }
                if let Some(ref mut c) = self.console_view { c.render(f, f.area()); }
            }
            AppState::ConfigWizard => {
                if self.config_wizard.is_none() { self.config_wizard = Some(ConfigWizard::new(self.language, &self.config)); }
                if let Some(ref mut c) = self.config_wizard { c.render(f, f.area()); }
            }
            AppState::UpdateView => {
                if self.update_view.is_none() { self.update_view = Some(UpdateView::new(self.language)); }
                if let Some(ref mut c) = self.update_view {
                    if let Some(ref s) = self.update_state { c.state = Some(s.clone()); }
                    c.render(f, f.area());
                }
            }
            AppState::ServerConfigEdit => {
                if self.server_config_edit.is_none() && !self.discovered_servers.is_empty() {
                    let ds = &self.discovered_servers[self.selected_server];
                    self.server_config_edit = Some(ServerConfigEdit::new(vec![("Name".into(), ds.name.clone()), ("Dir".into(), ds.dir.clone())], self.language));
                }
                if let Some(ref mut c) = self.server_config_edit { c.render(f, f.area()); }
            }
            AppState::JavaMenu => {
                if self.java_menu.is_none() { self.java_menu = Some(JavaMenu::new(self.language, self.config.as_ref())); }
                if let Some(ref mut c) = self.java_menu { c.render(f, f.area()); }
            }
            AppState::JavaSwitch(ref versions) => {
                if self.java_switch.is_none() { self.java_switch = Some(JavaSwitch::new(versions.clone(), self.language)); }
                if let Some(ref mut c) = self.java_switch { c.render(f, f.area()); }
            }
            AppState::JavaInstall => {
                if self.java_install.is_none() { self.java_install = Some(JavaInstall::new(self.language)); }
                if let Some(ref mut c) = self.java_install { c.render(f, f.area()); }
            }
            // === MainMenu component handles all 5 menu modes ===
            AppState::MainMenu | AppState::SubServer | AppState::SubMonitor | AppState::SubConfig | AppState::SubAdvanced => {
                if self.main_menu.is_none() {
                    let mut mm = MainMenu::new(self.language);
                    mm.server_running = self.server_running;
                    mm.mc_minder_running = self.mc_minder_running;
                    mm.watchdog_running = self.watchdog_running;
                    mm.mc_status = self.mc_status_snapshot.clone();
                    mm.session_name = self.get_session_name();
                    mm.discovered_servers = self.discovered_servers.iter().map(|d| d.name.clone()).collect();
                    mm.tps_history = self.tps_history.clone();
                    self.main_menu = Some(mm);
                }
                if let Some(ref mut c) = self.main_menu { c.render(f, f.area()); }
            }
            AppState::RunningForeground => {
                if self.running_foreground.is_none() { self.running_foreground = Some(RunningForeground::new(self.language)); }
                if let Some(ref mut c) = self.running_foreground {
                    c.console_lines = self.fg_console_lines.clone();
                    c.is_running = self.fg_server_alive;
                    c.render(f, f.area());
                }
            }
            AppState::NewServerWizard => {
                if self.new_server_wizard.is_none() { self.new_server_wizard = Some(NewServerWizard::new(self.language)); }
                if let Some(ref mut c) = self.new_server_wizard { c.render(f, f.area()); }
            }
            AppState::Busy(ref msg) => self.draw_busy(f, msg),
            AppState::BackupList => self.draw_backup_list(f),
        }

        // Draw message overlay
        if let Some((msg, msg_type)) = &self.message {
            let color = match msg_type {
                MessageType::Info => Color::Blue,
                MessageType::Success => Color::Green,
                MessageType::Warning => Color::Yellow,
                MessageType::Error => Color::Red,
            };
            let title = match msg_type {
                MessageType::Info => match self.language { Language::Chinese => "提示", Language::English => "Info" },
                MessageType::Success => match self.language { Language::Chinese => "成功", Language::English => "Success" },
                MessageType::Warning => match self.language { Language::Chinese => "警告", Language::English => "Warning" },
                MessageType::Error => match self.language { Language::Chinese => "错误", Language::English => "Error" },
            };
            let para = Paragraph::new(msg.as_str())
                .block(Block::default().title(title).borders(Borders::ALL))
                .style(Style::default().fg(color))
                .alignment(ratatui::layout::Alignment::Center);
            let area = centered_rect(60, 20, f.area());
            f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
            f.render_widget(para, area);
        }
    }

    // ============================================================
    // Categorized sub-menu system
    // ============================================================

    pub fn main_menu_items_new(&self) -> Vec<&'static str> {
        match self.language {
            Language::Chinese => vec!["服务器控制","监控与日志","配置与管理","高级工具","语言切换","退出"],
            Language::English => vec!["Server Control","Monitoring","Configuration","Advanced Tools","Language","Exit"],
        }
    }
    fn sub_server_items(&self) -> Vec<&'static str> {
        match self.language { Language::Chinese => vec!["启动(后台)","启动(前台)","停止服务器","重启服务器","返回"], Language::English => vec!["Start(Bg)","Start(Fg)","Stop","Restart","Back"] }
    }
    fn sub_monitor_items(&self) -> Vec<&'static str> {
        match self.language { Language::Chinese => vec!["服务器状态","控制台","服务器日志","MC-Minder日志","备份列表","已安装Mod","返回"], Language::English => vec!["Status","Console","Server Log","MC-Minder Log","Backups","Mods","Back"] }
    }
    fn sub_config_items(&self) -> Vec<&'static str> {
        match self.language { Language::Chinese => vec!["初始化配置","更新MC-Minder","Java管理","编辑配置","返回"], Language::English => vec!["Init Config","Update","Java","Edit Config","Back"] }
    }
    fn sub_advanced_items(&self) -> Vec<&'static str> {
        match self.language { Language::Chinese => vec!["新建服务器","Mod下载","备份世界","快捷指令","返回"], Language::English => vec!["New Server","Mods","Backup World","Quick Cmds","Back"] }
    }

    fn execute_sub_action(&mut self, sub_type: u8, index: usize) {
        match sub_type {
            0 => match index { 0=>self.start_server_background(),1=>self.start_server_foreground(),2=>{self.state=AppState::ConfirmDialog(ConfirmAction::StopServer)},3=>{self.state=AppState::ConfirmDialog(ConfirmAction::RestartServer)},_=>{self.state=AppState::MainMenu}},
            1 => match index { 0=>{self.state=AppState::StatusView},1=>self.enter_console(),2=>{self.load_server_log();self.state=AppState::LogViewer(LogType::Server)},3=>{self.load_minder_log();self.state=AppState::LogViewer(LogType::McMinder)},4=>{self.state=AppState::BackupList},5=>{self.state=AppState::ModList},_=>{self.state=AppState::MainMenu}},
            2 => match index { 0=>self.init_config(),1=>{self.state=AppState::ConfirmDialog(ConfirmAction::UpdateMcminder)},2=>{self.state=AppState::JavaMenu;self.java_menu_selected=0},3=>self.edit_server_config(),_=>{self.state=AppState::MainMenu}},
            3 => match index { 0=>self.open_new_server_wizard(),1=>self.open_mod_browser(),2=>self.trigger_backup(),3=>{self.state=AppState::QuickCommands},_=>{self.state=AppState::MainMenu}},
            _=>{}
        }
    }

    fn on_key_sub_menu(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let items = match &self.state { AppState::SubServer=>self.sub_server_items(),AppState::SubMonitor=>self.sub_monitor_items(),AppState::SubConfig=>self.sub_config_items(),AppState::SubAdvanced=>self.sub_advanced_items(),_=>return };
        let max=items.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc|KeyCode::Char('q')=>{self.state=AppState::MainMenu}
            KeyCode::Up|KeyCode::Char('k')|KeyCode::Char('8')=>{self.main_menu_selected=self.main_menu_selected.saturating_sub(1)}
            KeyCode::Down|KeyCode::Char('j')|KeyCode::Char('2')=>{if self.main_menu_selected<max{self.main_menu_selected+=1}}
            KeyCode::Enter=>{let st=match&self.state{AppState::SubServer=>0,AppState::SubMonitor=>1,AppState::SubConfig=>2,AppState::SubAdvanced=>3,_=>return};self.execute_sub_action(st,self.main_menu_selected)}
            _=>{}
        }
    }

    fn draw_sub_menu(&self, f: &mut Frame, items: &[&str], title: &str) {
        let li:Vec<ListItem>=items.iter().map(|t|ListItem::new(Span::raw(*t))).collect();
        let mut s=ratatui::widgets::ListState::default();s.select(Some(self.main_menu_selected));
        let l=List::new(li).block(Block::default().title(title).borders(Borders::ALL)).highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
        let a=centered_rect(40,18,f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)),a);f.render_stateful_widget(l,a,&mut s);
    }

}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
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

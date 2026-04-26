use std::path::PathBuf;
use std::fs;

use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Line};
use ratatui::Frame;

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
    // Update engine state
    #[allow(dead_code)]
    pub update_engine: UpdateEngine,
    pub update_rx: Option<tokio::sync::mpsc::Receiver<UpdateMsg>>,
    pub update_state: Option<UpdateState>,
}

pub enum AppState {
    MainMenu,
    JavaMenu,
    JavaSwitch(Vec<(String, String)>),  // (path, version) list for interactive selection
    JavaInstall,  // Java version installation picker
    LogViewer(LogType),
    ConfigWizard,
    LanguageSelect,
    ConfirmDialog(ConfirmAction),
    StatusView,
    Console,  // Real-time console output
    UpdateView,  // Update progress view
    #[allow(dead_code)]
    RunningForeground,  // Running foreground server inside TUI
}

// Update state machine
pub enum UpdateState {
    UpdateAvailable { current: String, latest: String, download_url: String },
    UpToDate,
    Downloading { downloaded: u64, total: Option<u64> },
    Installing,
    Done { new_version: String },
    Failed(String),
}

pub enum LogType {
    Server,
    McMinder,
}

pub enum ConfirmAction {
    StopServer,
    RestartServer,
    UpdateMcminder,
    Exit,
}

pub enum Language {
    Chinese,
    English,
}

pub enum MessageType {
    Info,
    Success,
    Warning,
    #[allow(dead_code)]
    Error,
}

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
            update_engine: UpdateEngine::new(),
            update_rx: None,
            update_state: None,
        }
    }

    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        match self.state {
            AppState::MainMenu => self.on_key_main_menu(key),
            AppState::JavaMenu => self.on_key_java_menu(key),
            AppState::JavaSwitch(_) => self.on_key_java_switch(key),
            AppState::JavaInstall => self.on_key_java_install(key),
            AppState::LogViewer(_) => self.on_key_log_viewer(key),
            AppState::ConfigWizard => self.on_key_config_wizard(key),
            AppState::LanguageSelect => self.on_key_language_select(key),
            AppState::ConfirmDialog(_) => self.on_key_confirm_dialog(key),
            AppState::StatusView => self.on_key_status_view(key),
            AppState::Console => self.on_key_console(key),
            AppState::UpdateView => self.on_key_update_view(key),
            AppState::RunningForeground => self.on_key_running_foreground(key),
        }
    }

    fn on_key_main_menu(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => { self.should_quit = true; }
            KeyCode::Down | KeyCode::Char('j') => {
                self.main_menu_selected = (self.main_menu_selected + 1) % 13;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.main_menu_selected = if self.main_menu_selected == 0 { 12 } else { self.main_menu_selected - 1 };
            }
            KeyCode::Enter => {
                self.execute_main_menu_action(self.main_menu_selected);
            }
            KeyCode::Char('1') => self.execute_main_menu_action(0),
            KeyCode::Char('2') => self.execute_main_menu_action(1),
            KeyCode::Char('3') => self.execute_main_menu_action(2),
            KeyCode::Char('4') => self.execute_main_menu_action(3),
            KeyCode::Char('5') => self.execute_main_menu_action(4),
            KeyCode::Char('6') => self.execute_main_menu_action(5),
            KeyCode::Char('7') => self.execute_main_menu_action(6),
            KeyCode::Char('8') => self.execute_main_menu_action(7),
            KeyCode::Char('9') => self.execute_main_menu_action(8),
            _ => {}
        }
    }

    fn on_key_java_menu(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.state = AppState::MainMenu; }
            KeyCode::Down | KeyCode::Char('j') => {
                self.java_menu_selected = (self.java_menu_selected + 1) % 4;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.java_menu_selected = if self.java_menu_selected == 0 { 3 } else { self.java_menu_selected - 1 };
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
                rcon: crate::config::RconConfig { host: "127.0.0.1".to_string(), port: 25575, password: String::new() },
                server: crate::config::ServerConfig::default(),
                backup: crate::config::BackupConfig::default(),
                notification: crate::config::NotificationConfig::default(),
                jvm: crate::config::JvmConfig::default(),
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
        let session = self.get_session_name();
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
            0 => self.start_server_background(),
            1 => self.start_server_foreground(),
            2 => {
                self.state = AppState::ConfirmDialog(ConfirmAction::StopServer);
            }
            3 => {
                self.state = AppState::ConfirmDialog(ConfirmAction::RestartServer);
            }
            4 => self.show_server_status(),
            5 => self.enter_console(),  // Changed from attach_server_console
            6 => {
                self.load_server_log();
                self.state = AppState::LogViewer(LogType::Server);
            }
            7 => {
                self.load_minder_log();
                self.state = AppState::LogViewer(LogType::McMinder);
            }
            8 => self.init_config(),
            9 => {
                self.state = AppState::ConfirmDialog(ConfirmAction::UpdateMcminder);
            }
            10 => {
                self.state = AppState::JavaMenu;
                self.java_menu_selected = 0;
            }
            11 => {
                self.state = AppState::LanguageSelect;
            }
            12 => {
                self.state = AppState::ConfirmDialog(ConfirmAction::Exit);
            }
            _ => {}
        }
    }

    fn execute_confirm_action(&mut self) {
        // Store action before changing state
        let action = match &self.state {
            AppState::ConfirmDialog(a) => match a {
                ConfirmAction::StopServer => Some("stop"),
                ConfirmAction::RestartServer => Some("restart"),
                ConfirmAction::UpdateMcminder => Some("update"),
                ConfirmAction::Exit => Some("exit"),
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
                _ => {}
            }
        }
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
        let versions = self.detect_java_versions();
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
        let versions = self.detect_java_versions();
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
    fn get_session_name(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.session_name.clone())
            .unwrap_or_else(|| "mc_server".to_string())
    }

    pub(crate) fn get_jar(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.jar.clone())
            .unwrap_or_else(|| "fabric-server.jar".to_string())
    }

    pub(crate) fn get_min_mem(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.min_mem.clone())
            .unwrap_or_else(|| "512M".to_string())
    }

    pub(crate) fn get_max_mem(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.max_mem.clone())
            .unwrap_or_else(|| "1G".to_string())
    }

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
        match self.language {
            Language::Chinese => vec![
                "1. 启动服务器（后台模式）",
                "2. 启动服务器（前台模式）",
                "3. 停止服务器",
                "4. 重启服务器",
                "5. 查看服务器状态",
                "6. 实时控制台视图",
                "7. 查看服务器日志",
                "8. 查看 MC-Minder 日志",
                "9. 初始化配置",
                "10. 更新 MC-Minder",
                "11. Java 版本管理",
                "12. 语言设置",
                "13. 退出",
            ],
            Language::English => vec![
                "1. Start Server (Background)",
                "2. Start Server (Foreground)",
                "3. Stop Server",
                "4. Restart Server",
                "5. View Server Status",
                "6. Real-time Console View",
                "7. View Server Log",
                "8. View MC-Minder Log",
                "9. Initialize Config",
                "10. Update MC-Minder",
                "11. Java Version Management",
                "12. Language Settings",
                "13. Exit",
            ],
        }
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
        let list_items: Vec<ListItem> = menu_items.iter()
            .map(|i| ListItem::new(Span::raw(*i)))
            .collect();
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
        let versions = self.detect_java_versions();
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
        let options = self.java_install_options();
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
        };

        let para = Paragraph::new(msg)
            .block(Block::default().title(title).borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center);
        let area = centered_rect(50, 30, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_widget(para, area);
    }

    fn draw_status_view(&self, f: &mut Frame) {
        let title = match self.language {
            Language::Chinese => "服务器状态",
            Language::English => "Server Status",
        };

        let session = self.get_session_name();
        let status = format!(
            "tmux 会话: {}\n  状态: {}\n\nMC-Minder:\n  状态: {}\n\n看门狗:\n  状态: {}\n\n配置文件: {}",
            session,
            if self.server_running { "运行中" } else { "未运行" },
            if self.mc_minder_running { "运行中" } else { "未运行" },
            if self.watchdog_running { "运行中" } else { "未运行" },
            self.config_path.display(),
        );

        let para = Paragraph::new(status)
            .block(Block::default().title(title).borders(Borders::ALL));
        f.render_widget(para, f.area());

        let help = match self.language {
            Language::Chinese => "按 Enter 或 Esc 返回主菜单",
            Language::English => "Press Enter or Esc to return to main menu",
        };
        let help_block = Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help_block, Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.area())[1]);
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
        let lang = match self.language {
            Language::Chinese => "中文",
            Language::English => "English",
        };
        let session = self.get_session_name();
        format!(
            "语言: {}\n配置: {}\n\n服务器:\n  会话: {}\n  状态: {}\n\nMC-Minder:\n  状态: {}\n\n看门狗:\n  状态: {}",
            lang,
            self.config_path.display(),
            session,
            if self.server_running { "运行中" } else { "未运行" },
            if self.mc_minder_running { "运行中" } else { "未运行" },
            if self.watchdog_running { "运行中" } else { "未运行" },
        )
    }

    pub fn draw(&mut self, f: &mut Frame) {
        // Check for message timeout
        if let Some(timeout) = &self.message_timeout {
            if timeout.elapsed() > std::time::Duration::from_secs(3) {
                // Message expired - handle in event loop
            }
        }

        match &self.state {
            AppState::MainMenu => self.draw_main_menu(f),
            AppState::JavaMenu => self.draw_java_menu(f),
            AppState::JavaSwitch(versions) => self.draw_java_switch(f, versions),
            AppState::JavaInstall => self.draw_java_install(f),
            AppState::LogViewer(log_type) => self.draw_log_viewer(f, log_type),
            AppState::ConfigWizard => self.draw_config_wizard(f),
            AppState::LanguageSelect => self.draw_language_select(f),
            AppState::ConfirmDialog(action) => self.draw_confirm_dialog(f, action),
            AppState::StatusView => self.draw_status_view(f),
            AppState::Console => self.draw_console(f),
            AppState::UpdateView => self.draw_update_view(f),
            AppState::RunningForeground => self.draw_running_foreground(f),
        }

        // Draw message overlay if present
        if let Some((msg, msg_type)) = &self.message {
            let color = match msg_type {
                MessageType::Info => Color::Blue,
                MessageType::Success => Color::Green,
                MessageType::Warning => Color::Yellow,
                MessageType::Error => Color::Red,
            };
            let title = match msg_type {
                MessageType::Info => match self.language {
                    Language::Chinese => "提示",
                    Language::English => "Info",
                },
                MessageType::Success => match self.language {
                    Language::Chinese => "成功",
                    Language::English => "Success",
                },
                MessageType::Warning => match self.language {
                    Language::Chinese => "警告",
                    Language::English => "Warning",
                },
                MessageType::Error => match self.language {
                    Language::Chinese => "错误",
                    Language::English => "Error",
                },
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

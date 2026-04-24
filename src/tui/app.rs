use std::path::PathBuf;

use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

use crate::config::Config;

pub struct App {
    pub state: AppState,
    pub should_quit: bool,
    pub language: Language,
    pub config_path: PathBuf,
    pub config: Option<Config>,
    pub main_menu_selected: usize,
    pub java_menu_selected: usize,
    pub server_running: bool,
    pub mc_minder_running: bool,
    pub watchdog_running: bool,
    pub server_log_content: String,
    pub minder_log_content: String,
    pub log_scroll: usize,
    pub message: Option<(String, MessageType)>,
    pub message_timeout: Option<std::time::Instant>,
}

pub enum AppState {
    MainMenu,
    JavaMenu,
    LogViewer(LogType),
    ConfigWizard,
    LanguageSelect,
    ConfirmDialog(ConfirmAction),
    StatusView,
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
    Error,
}

impl App {
    pub fn new(config_path: PathBuf) -> Self {
        let cfg = Config::load(&config_path).ok();
        App {
            state: AppState::MainMenu,
            should_quit: false,
            language: Language::Chinese,
            config_path,
            config: cfg,
            main_menu_selected: 0,
            java_menu_selected: 0,
            server_running: false,
            mc_minder_running: false,
            watchdog_running: false,
            server_log_content: String::new(),
            minder_log_content: String::new(),
            log_scroll: 0,
            message: None,
            message_timeout: None,
        }
    }

    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        match self.state {
            AppState::MainMenu => self.on_key_main_menu(key),
            AppState::JavaMenu => self.on_key_java_menu(key),
            AppState::LogViewer(_) => self.on_key_log_viewer(key),
            AppState::ConfigWizard => self.on_key_config_wizard(key),
            AppState::LanguageSelect => self.on_key_language_select(key),
            AppState::ConfirmDialog(_) => self.on_key_confirm_dialog(key),
            AppState::StatusView => self.on_key_status_view(key),
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
            KeyCode::Tab => { /* next field */ }
            KeyCode::BackTab => { /* prev field */ }
            KeyCode::Enter => { /* save config */ self.state = AppState::MainMenu; }
            _ => {}
        }
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
            5 => self.attach_server_console(),
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
    }

    fn start_server_foreground(&mut self) {
        let _jar = self.get_jar();
        let _min_mem = self.get_min_mem();
        let _max_mem = self.get_max_mem();

        self.message = Some((
            if matches!(self.language, Language::Chinese) {
                "前台模式需要直接运行: java -Xms{} -Xmx{} -jar {} nogui".to_string()
            } else {
                "Foreground mode requires running directly: java -Xms{} -Xmx{} -jar {} nogui".to_string()
            },
            MessageType::Info,
        ));
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

    fn attach_server_console(&mut self) {
        let session = self.get_session_name();
        self.message = Some((
            if matches!(self.language, Language::Chinese) {
                format!("请在终端中运行: tmux attach -t {}", session)
            } else {
                format!("Run in terminal: tmux attach -t {}", session)
            },
            MessageType::Info,
        ));
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
        let bin = self.find_mcminder_bin();
        let output = std::process::Command::new(&bin)
            .arg("self-update")
            .output();

        match output {
            Ok(out) => {
                let msg = String::from_utf8_lossy(&out.stdout).to_string();
                self.message = Some((
                    if matches!(self.language, Language::Chinese) {
                        format!("更新完成: {}", msg)
                    } else {
                        format!("Update complete: {}", msg)
                    },
                    MessageType::Success,
                ));
            }
            Err(e) => {
                self.message = Some((
                    if matches!(self.language, Language::Chinese) {
                        format!("更新失败: {}", e)
                    } else {
                        format!("Update failed: {}", e)
                    },
                    MessageType::Error,
                ));
            }
        }
    }

    fn switch_java_version(&mut self) {
        // Detect Java versions
        let versions = self.detect_java_versions();
        if versions.is_empty() {
            self.message = Some((
                if matches!(self.language, Language::Chinese) {
                    "未检测到 Java 版本".to_string()
                } else {
                    "No Java versions detected".to_string()
                },
                MessageType::Warning,
            ));
            return;
        }
        // For now, show first version found
        self.message = Some((
            if matches!(self.language, Language::Chinese) {
                format!("检测到 {} 个 Java 版本，请使用 mc-minder init 配置", versions.len())
            } else {
                format!("Detected {} Java versions, use mc-minder init to configure", versions.len())
            },
            MessageType::Info,
        ));
    }

    fn install_java_version(&mut self) {
        self.message = Some((
            if matches!(self.language, Language::Chinese) {
                "请在 Termux 中运行: pkg install openjdk-17".to_string()
            } else {
                "Run in Termux: pkg install openjdk-17".to_string()
            },
            MessageType::Info,
        ));
    }

    fn show_installed_java(&mut self) {
        let versions = self.detect_java_versions();
        let msg = if versions.is_empty() {
            if matches!(self.language, Language::Chinese) {
                "未检测到 Java 版本".to_string()
            } else {
                "No Java versions detected".to_string()
            }
        } else {
            versions.join("\n")
        };
        self.message = Some((msg, MessageType::Info));
    }

    fn detect_java_versions(&self) -> Vec<String> {
        let mut versions = Vec::new();

        // Check java command
        if let Ok(out) = std::process::Command::new("java").arg("-version").output() {
            let ver = String::from_utf8_lossy(&out.stderr);
            if let Some(line) = ver.lines().next() {
                versions.push(line.to_string());
            }
        }

        // Check common paths
        let paths = if cfg!(target_os = "android") {
            vec!["/data/data/com.termux/files/usr/lib/jvm"]
        } else {
            vec!["/usr/lib/jvm", "/usr/java", "/opt/java"]
        };

        for base in paths {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.contains("jdk") || name.contains("jre") || name.contains("openjdk") {
                            versions.push(format!("{}/{}", base, name));
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
    }

    // Helper methods
    fn get_session_name(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.session_name.clone())
            .unwrap_or_else(|| "mc_server".to_string())
    }

    fn get_jar(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.jar.clone())
            .unwrap_or_else(|| "fabric-server.jar".to_string())
    }

    fn get_min_mem(&self) -> String {
        self.config.as_ref()
            .map(|c| c.server.min_mem.clone())
            .unwrap_or_else(|| "512M".to_string())
    }

    fn get_max_mem(&self) -> String {
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
                "6. 附加到服务器控制台",
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
                "6. Attach to Server Console",
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
            versions.join("\n")
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
        let title = match self.language {
            Language::Chinese => "配置向导",
            Language::English => "Configuration Wizard",
        };
        let msg = match self.language {
            Language::Chinese => "配置向导尚未完全实现\n\n请使用命令行: mc-minder init",
            Language::English => "Config wizard not fully implemented\n\nUse command line: mc-minder init",
        };
        let para = Paragraph::new(msg)
            .block(Block::default().title(title).borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center);
        let area = centered_rect(60, 30, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black)), area);
        f.render_widget(para, area);
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

    pub fn draw(&self, f: &mut Frame) {
        // Check for message timeout
        if let Some(timeout) = &self.message_timeout {
            if timeout.elapsed() > std::time::Duration::from_secs(3) {
                // Message expired - handle in event loop
            }
        }

        match &self.state {
            AppState::MainMenu => self.draw_main_menu(f),
            AppState::JavaMenu => self.draw_java_menu(f),
            AppState::LogViewer(log_type) => self.draw_log_viewer(f, log_type),
            AppState::ConfigWizard => self.draw_config_wizard(f),
            AppState::LanguageSelect => self.draw_language_select(f),
            AppState::ConfirmDialog(action) => self.draw_confirm_dialog(f, action),
            AppState::StatusView => self.draw_status_view(f),
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

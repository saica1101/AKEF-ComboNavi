//! AKEF ComboNavi - Arknights: Endfield Combo Navigation Tool

pub mod combo;
pub mod config;
pub mod input;
pub mod process;

use combo::ComboFile;
use config::Config;
use input::{InputHandler, KeyEvent};
use process::ProcessMonitor;
use rdev::Key;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};

pub struct AppState {
    pub combo_file: RwLock<Option<ComboFile>>,
    pub current_index: RwLock<usize>,
    pub config: RwLock<Config>,
    pub process_monitor: RwLock<ProcessMonitor>,
    pub input_handler: InputHandler,
    pub overlay_visible: RwLock<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            combo_file: RwLock::new(None),
            current_index: RwLock::new(0),
            config: RwLock::new(Config::load_or_default()),
            process_monitor: RwLock::new(ProcessMonitor::new()),
            input_handler: InputHandler::new(),
            overlay_visible: RwLock::new(true),
        }
    }

    pub fn sync_input_handler(&self) {
        let combo = self.combo_file.read();
        let index = *self.current_index.read();

        let command = if let Some(ref file) = *combo {
            let commands: Vec<_> = file.commands.iter().filter(|c| !c.is_title).collect();
            if index < commands.len() {
                Some(commands[index].clone())
            } else {
                None
            }
        } else {
            None
        };

        self.input_handler.set_current_command(command);
    }

    fn get_current_command_internal(&self) -> Option<CurrentCommandInfo> {
        let combo = self.combo_file.read();
        let index = *self.current_index.read();

        if let Some(ref file) = *combo {
            let commands: Vec<_> = file.commands.iter().filter(|c| !c.is_title).collect();

            if index < commands.len() {
                let cmd = &commands[index];
                let key_display = match &cmd.key {
                    combo::KeyIdentifier::Number(n) => {
                        if matches!(cmd.input_type, combo::InputType::Hold { .. }) {
                            format!("Hold {}", n)
                        } else {
                            n.to_string()
                        }
                    }
                    combo::KeyIdentifier::Chain => "E".to_string(),
                    combo::KeyIdentifier::HeavyAttack | combo::KeyIdentifier::MouseLeft => {
                        "L".to_string()
                    }
                };

                return Some(CurrentCommandInfo {
                    index,
                    total: commands.len(),
                    title: file.title.clone(),
                    key_display,
                    character: cmd.character.clone(),
                    skill_type: cmd.skill_type.clone(),
                    memo: cmd.memo.clone(),
                    is_hold: matches!(cmd.input_type, combo::InputType::Hold { .. }),
                });
            }
        }

        None
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentCommandInfo {
    pub index: usize,
    pub total: usize,
    pub title: String,
    pub key_display: String,
    pub character: String,
    pub skill_type: String,
    pub memo: String,
    pub is_hold: bool,
}

// ============= Tauri Commands =============

#[tauri::command]
fn load_combo_file(
    path: String,
    state: State<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let combo_result = combo::parse_combo_file(&path).map_err(|e| e.to_string())?;
    let title = combo_result.title.clone();

    *state.combo_file.write() = Some(combo_result);
    *state.current_index.write() = 0;
    state.sync_input_handler();

    {
        let mut config = state.config.write();
        config.last_combo_file = Some(path);
        let _ = config.save(Config::default_path());
    }

    if let Some(cmd) = state.get_current_command_internal() {
        let _ = app_handle.emit("combo-update", cmd);
    }

    Ok(title)
}

#[tauri::command]
fn get_current_command(state: State<AppState>) -> Option<CurrentCommandInfo> {
    state.get_current_command_internal()
}

#[tauri::command]
fn advance_command(state: State<AppState>) -> Option<CurrentCommandInfo> {
    {
        let combo = state.combo_file.read();
        if let Some(ref file) = *combo {
            let commands: Vec<_> = file.commands.iter().filter(|c| !c.is_title).collect();
            let mut index = state.current_index.write();

            if commands.is_empty() {
                return None;
            }
            *index = (*index + 1) % commands.len();
        }
    }
    state.sync_input_handler();
    state.get_current_command_internal()
}

#[tauri::command]
fn previous_command(state: State<AppState>) -> Option<CurrentCommandInfo> {
    {
        let combo = state.combo_file.read();
        if let Some(ref file) = *combo {
            let commands: Vec<_> = file.commands.iter().filter(|c| !c.is_title).collect();
            let mut index = state.current_index.write();
            if !commands.is_empty() {
                if *index > 0 {
                    *index -= 1;
                }
            }
        }
    }
    state.sync_input_handler();
    state.get_current_command_internal()
}

#[tauri::command]
fn reset_combo(state: State<AppState>) -> Option<CurrentCommandInfo> {
    *state.current_index.write() = 0;
    state.sync_input_handler();
    state.get_current_command_internal()
}

#[tauri::command]
fn get_config(state: State<AppState>) -> config::Config {
    state.config.read().clone()
}

#[tauri::command]
fn save_config(new_config: config::Config, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.write();
    *config = new_config;
    config
        .save(Config::default_path())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn is_game_running() -> bool {
    ProcessMonitor::check_once()
}

#[tauri::command]
fn toggle_overlay(state: State<AppState>, app_handle: tauri::AppHandle) -> bool {
    let mut visible = state.overlay_visible.write();
    *visible = !*visible;

    if let Some(window) = app_handle.get_webview_window("main") {
        if *visible {
            let _ = window.show();
        } else {
            let _ = window.hide();
        }
    }

    *visible
}

#[tauri::command]
fn set_overlay_visible(visible: bool, state: State<AppState>) {
    *state.overlay_visible.write() = visible;
}

#[tauri::command]
fn get_overlay_visible(state: State<AppState>) -> bool {
    *state.overlay_visible.read()
}

#[tauri::command]
async fn open_settings_window(app_handle: tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
async fn set_overlay_opacity(app_handle: tauri::AppHandle, opacity: f64) -> Result<(), String> {
    let _ = app_handle.emit("overlay-opacity-changed", opacity);
    Ok(())
}

#[tauri::command]
async fn app_exit(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
async fn set_click_through(window: tauri::Window, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        window
            .set_ignore_cursor_events(enabled)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn key_to_string(key: Key) -> String {
    match key {
        Key::KeyA => "A".to_string(),
        Key::KeyB => "B".to_string(),
        Key::KeyC => "C".to_string(),
        Key::KeyD => "D".to_string(),
        Key::KeyE => "E".to_string(),
        Key::KeyF => "F".to_string(),
        Key::KeyG => "G".to_string(),
        Key::KeyH => "H".to_string(),
        Key::KeyI => "I".to_string(),
        Key::KeyJ => "J".to_string(),
        Key::KeyK => "K".to_string(),
        Key::KeyL => "L".to_string(),
        Key::KeyM => "M".to_string(),
        Key::KeyN => "N".to_string(),
        Key::KeyO => "O".to_string(),
        Key::KeyP => "P".to_string(),
        Key::KeyQ => "Q".to_string(),
        Key::KeyR => "R".to_string(),
        Key::KeyS => "S".to_string(),
        Key::KeyT => "T".to_string(),
        Key::KeyU => "U".to_string(),
        Key::KeyV => "V".to_string(),
        Key::KeyW => "W".to_string(),
        Key::KeyX => "X".to_string(),
        Key::KeyY => "Y".to_string(),
        Key::KeyZ => "Z".to_string(),
        Key::Num1 => "1".to_string(),
        Key::Num2 => "2".to_string(),
        Key::Num3 => "3".to_string(),
        Key::Num4 => "4".to_string(),
        Key::Num5 => "5".to_string(),
        Key::Num6 => "6".to_string(),
        Key::Num7 => "7".to_string(),
        Key::Num8 => "8".to_string(),
        Key::Num9 => "9".to_string(),
        Key::Num0 => "0".to_string(),
        Key::Space => "Space".to_string(),
        Key::Return => "Enter".to_string(),
        Key::Escape => "Escape".to_string(),
        Key::Tab => "Tab".to_string(),
        _ => format!("{:?}", key),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            load_combo_file,
            get_current_command,
            advance_command,
            previous_command,
            reset_combo,
            get_config,
            save_config,
            is_game_running,
            toggle_overlay,
            set_overlay_visible,
            get_overlay_visible,
            set_click_through,
            open_settings_window,
            set_overlay_opacity,
            app_exit,
        ])
        .setup(|app| {
            // Set initial click-through state for main window
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.set_ignore_cursor_events(true);
            }

            if let Some(settings_window) = app.get_webview_window("settings") {
                let settings_clone = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_clone.minimize();
                    }
                });
            }

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_status = false;
                loop {
                    let running = ProcessMonitor::check_once();
                    if running != last_status {
                        last_status = running;
                        let _ = app_handle.emit("game-status-changed", running);
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            });

            let app_handle_input = app.handle().clone();
            let input_handler = app.state::<AppState>().input_handler.clone();

            std::thread::spawn(move || {
                let mut rx = input::start_global_key_listener(input_handler);

                while let Some(event) = rx.blocking_recv() {
                    let state = app_handle_input.state::<AppState>();

                    match event {
                        KeyEvent::TapComplete(_) | KeyEvent::HoldComplete(_) => {
                            let mut advanced = false;
                            {
                                let combo = state.combo_file.read();
                                if let Some(ref file) = *combo {
                                    let commands: Vec<_> =
                                        file.commands.iter().filter(|c| !c.is_title).collect();
                                    let mut index = state.current_index.write();
                                    if !commands.is_empty() {
                                        *index = (*index + 1) % commands.len();
                                        advanced = true;
                                    }
                                }
                            }

                            if advanced {
                                state.sync_input_handler();
                                if let Some(cmd) = state.get_current_command_internal() {
                                    let _ = app_handle_input.emit("combo-update", cmd);
                                }
                            }
                        }
                        KeyEvent::HoldProgress(_, progress) => {
                            // Emit hold progress to frontend
                            let _ = app_handle_input.emit("hold-progress", progress);
                        }
                        KeyEvent::KeyDown(key) => {
                            if matches!(key, Key::Alt | Key::AltGr) {
                                println!("[DEBUG] lib.rs received Alt KeyDown: {:?}", key);
                                let _ = app_handle_input.emit("alt-status-changed", true);
                                if let Some(win) = app_handle_input.get_webview_window("main") {
                                    let _ = win.set_ignore_cursor_events(false);
                                }
                            }

                            match key {
                                Key::RightArrow => {
                                    let mut advanced = false;
                                    {
                                        let combo = state.combo_file.read();
                                        if let Some(ref file) = *combo {
                                            let commands: Vec<_> = file
                                                .commands
                                                .iter()
                                                .filter(|c| !c.is_title)
                                                .collect();
                                            let mut index = state.current_index.write();
                                            if !commands.is_empty() {
                                                *index = (*index + 1) % commands.len();
                                                advanced = true;
                                            }
                                        }
                                    }
                                    if advanced {
                                        state.sync_input_handler();
                                        if let Some(cmd) = state.get_current_command_internal() {
                                            let _ = app_handle_input.emit("combo-update", cmd);
                                        }
                                    }
                                }
                                Key::LeftArrow => {
                                    let mut changed = false;
                                    {
                                        let combo = state.combo_file.read();
                                        if let Some(ref file) = *combo {
                                            let commands: Vec<_> = file
                                                .commands
                                                .iter()
                                                .filter(|c| !c.is_title)
                                                .collect();
                                            let mut index = state.current_index.write();
                                            if !commands.is_empty() && *index > 0 {
                                                *index -= 1;
                                                changed = true;
                                            }
                                        }
                                    }
                                    if changed {
                                        state.sync_input_handler();
                                        if let Some(cmd) = state.get_current_command_internal() {
                                            let _ = app_handle_input.emit("combo-update", cmd);
                                        }
                                    }
                                }
                                _ => {}
                            }

                            // Hotkey Check
                            let config = state.config.read();
                            let key_str = key_to_string(key);

                            println!("[DEBUG] Key pressed: {:?} => '{}'", key, key_str);
                            println!(
                                "[DEBUG] open_settings binding: '{}'",
                                config.key_bindings.open_settings
                            );

                            if key_str == config.key_bindings.open_settings {
                                println!("[DEBUG] Opening settings window");
                                let _ = app_handle_input.emit("request-open-settings", ());
                                // Drop config lock before window operations
                                drop(config);

                                if let Some(window) =
                                    app_handle_input.get_webview_window("settings")
                                {
                                    // Ensure window is visible and focused
                                    let _ = window.show();
                                    let _ = window.unminimize();
                                    let _ = window.set_focus();
                                }
                            } else if key_str == config.key_bindings.toggle_overlay {
                                let mut visible = state.overlay_visible.write();
                                *visible = !*visible;

                                if let Some(window) = app_handle_input.get_webview_window("main") {
                                    if *visible {
                                        let _ = window.show();
                                    } else {
                                        let _ = window.hide();
                                    }
                                }

                                let _ =
                                    app_handle_input.emit("overlay-visibility-changed", *visible);
                            }
                        }
                        KeyEvent::KeyUp(key) => {
                            if matches!(key, Key::Alt | Key::AltGr) {
                                println!("[DEBUG] lib.rs received Alt KeyUp: {:?}", key);
                                let _ = app_handle_input.emit("alt-status-changed", false);
                                if let Some(win) = app_handle_input.get_webview_window("main") {
                                    let _ = win.set_ignore_cursor_events(true);
                                }
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

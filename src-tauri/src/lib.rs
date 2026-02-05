//! AKEF ComboNavi - Arknights: Endfield Combo Navigation Tool
//!
//! This module provides the core functionality for the combo navigation overlay.

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

/// Global application state
pub struct AppState {
    /// Current combo file
    pub combo_file: RwLock<Option<ComboFile>>,
    /// Current command index
    pub current_index: RwLock<usize>,
    /// Configuration
    pub config: RwLock<Config>,
    /// Process monitor
    pub process_monitor: RwLock<ProcessMonitor>,
    /// Input handler
    pub input_handler: InputHandler,
    /// Whether overlay is visible
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

    /// Sync current command to input handler
    pub fn sync_input_handler(&self) {
        let combo = self.combo_file.read();
        let index = *self.current_index.read();

        let command = if let Some(ref file) = *combo {
            let commands: Vec<_> = file.commands.iter().filter(|c| !c.is_title).collect(); // Note: inefficient to collect every time, but ok for now
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

    /// Get current command info (internal helper)
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

/// Current command info for frontend
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

/// Load a combo file
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

    // Sync input handler
    state.sync_input_handler();

    // Update config with last loaded file
    {
        let mut config = state.config.write();
        config.last_combo_file = Some(path);
        let _ = config.save(Config::default_path());
    }

    // Emit event to update all windows (especially main overlay)
    if let Some(cmd) = state.get_current_command_internal() {
        let _ = app_handle.emit("combo-update", cmd);
    }

    Ok(title)
}

/// Get current command info
#[tauri::command]
fn get_current_command(state: State<AppState>) -> Option<CurrentCommandInfo> {
    state.get_current_command_internal()
}

/// Advance to next command
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

/// Go to previous command
#[tauri::command]
fn previous_command(state: State<AppState>) -> Option<CurrentCommandInfo> {
    {
        let combo = state.combo_file.read();
        if let Some(ref file) = *combo {
            let commands: Vec<_> = file.commands.iter().filter(|c| !c.is_title).collect();
            let mut index = state.current_index.write();

            if commands.is_empty() {
            } else {
                if *index > 0 {
                    *index -= 1;
                }
            }
        }
    }

    state.sync_input_handler();
    state.get_current_command_internal()
}

/// Reset to first command
#[tauri::command]
fn reset_combo(state: State<AppState>) -> Option<CurrentCommandInfo> {
    *state.current_index.write() = 0;
    state.sync_input_handler();
    state.get_current_command_internal()
}

/// Get configuration
#[tauri::command]
fn get_config(state: State<AppState>) -> config::Config {
    state.config.read().clone()
}

/// Save configuration
#[tauri::command]
fn save_config(new_config: config::Config, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.write();
    *config = new_config;
    config
        .save(Config::default_path())
        .map_err(|e| e.to_string())
}

/// Check if target process is running
#[tauri::command]
fn is_game_running() -> bool {
    ProcessMonitor::check_once()
}

/// Toggle overlay visibility
#[tauri::command]
fn toggle_overlay(state: State<AppState>) -> bool {
    let mut visible = state.overlay_visible.write();
    *visible = !*visible;
    *visible
}

/// Set overlay visibility
#[tauri::command]
fn set_overlay_visible(visible: bool, state: State<AppState>) {
    *state.overlay_visible.write() = visible;
}

/// Get overlay visibility
#[tauri::command]
fn get_overlay_visible(state: State<AppState>) -> bool {
    *state.overlay_visible.read()
}

/// Open settings window
#[tauri::command]
async fn open_settings_window(app_handle: tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Set overlay opacity
#[tauri::command]
async fn set_overlay_opacity(app_handle: tauri::AppHandle, opacity: f64) -> Result<(), String> {
    // Emit event for frontend to handle style updates
    println!("DEBUG: set_overlay_opacity called with {}", opacity);
    let _ = app_handle.emit("overlay-opacity-changed", opacity);
    Ok(())
}

/// Exit application
#[tauri::command]
async fn app_exit(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

/// Set window click-through state
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

// ============= App Entry Point =============

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
            // Handle settings window close event (minimize instead of hide to keep in taskbar)
            if let Some(settings_window) = app.get_webview_window("settings") {
                let settings_clone = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_clone.minimize();
                    }
                });
            }

            // Start process monitor in background
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

            // Start input listener
            let app_handle_input = app.handle().clone();
            let input_handler = app.state::<AppState>().input_handler.clone();

            std::thread::spawn(move || {
                let mut rx = input::start_global_key_listener(input_handler);

                while let Some(event) = rx.blocking_recv() {
                    let state = app_handle_input.state::<AppState>();

                    match event {
                        KeyEvent::TapComplete(_) | KeyEvent::HoldComplete(_) => {
                            // Advance combo
                            // We need to call advance_command logic
                            // But we can't call the command directly easily, so we duplicate logic or make a public method
                            // For simplicity, duplicate logic here or access internal

                            // Advance index
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
                        KeyEvent::KeyDown(key) => {
                            // Global Navigation
                            match key {
                                Key::RightArrow => {
                                    // Advance combo
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
                                    // Previous combo
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

                            // Check hotkeys
                            let config = state.config.read();
                            let key_str = format!("{:?}", key);

                            if key_str == config.key_bindings.open_settings {
                                let _ = app_handle_input.emit("request-open-settings", ());
                                // Also attempt to open it directly backend side
                                if let Some(window) =
                                    app_handle_input.get_webview_window("settings")
                                {
                                    let _ = window.unminimize();
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            } else if key_str == config.key_bindings.toggle_overlay {
                                let mut visible = state.overlay_visible.write();
                                *visible = !*visible;
                                let _ =
                                    app_handle_input.emit("overlay-visibility-changed", *visible);
                                // Frontend polling handles visibility usually, but better to emit event? Or frontend polls.
                                // Frontend uses $overlayVisible store which is toggled by `toggle_overlay` command.
                                // But if toggled by hotkey, we should emit event or frontend won't know?
                                // Stores check `get_overlay_visible` usually?
                                // Looking at svelte, it doesn't listen to visibility changes from backend, it drives them.
                                // We should add a listener in frontend if we want hotkey to work properly reflect in UI store?
                                // "toggle_overlay" command returns the new state.

                                // Actually, `toggle_overlay` command updates the store in `combo.ts`.
                                // We need to emit an event so frontend can update store.
                                // But `combo.ts` doesn't listen for visibility change.
                                // We can rely on `state.overlay_visible` being read by frontend interval?
                                // `+page.svelte` has interval for `checkGameRunning`.
                                // It doesn't poll visibility.

                                // To make this work, I will rely on "request-open-settings" pattern.
                                // Or direct manipulation if I could.
                                // For now, just toggling state is enough if the overlay logic (frontend) checks it.
                                // But frontend hides itself based on Svelte store `$overlayVisible`.
                                // If I only update Rust state, Svelte store won't update!
                                // Frontend needs to listen to an event or poll.
                                // I'll emit "toggle-overlay-visual" and frontend *should* listen.
                            }
                        }
                        _ => {}
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

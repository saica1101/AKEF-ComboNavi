//! Process monitoring module
//!
//! Monitors for the Endfield.exe process to control overlay visibility.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use sysinfo::System;

/// Target process name to monitor
const TARGET_PROCESS: &str = "Endfield.exe";

/// Process monitor state
pub struct ProcessMonitor {
    /// Whether the target process is currently running
    is_running: Arc<AtomicBool>,
    /// Handle to the monitoring thread
    _thread_handle: Option<thread::JoinHandle<()>>,
    /// Stop flag for the monitoring thread
    stop_flag: Arc<AtomicBool>,
}

impl ProcessMonitor {
    /// Create a new process monitor
    pub fn new() -> Self {
        let is_running = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::new(AtomicBool::new(false));

        Self {
            is_running,
            _thread_handle: None,
            stop_flag,
        }
    }

    /// Start monitoring for the target process
    pub fn start(&mut self) {
        let is_running = self.is_running.clone();
        let stop_flag = self.stop_flag.clone();

        let handle = thread::spawn(move || {
            let mut system = System::new();

            while !stop_flag.load(Ordering::Relaxed) {
                // Refresh process list
                system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

                // Check if target process is running
                let found = system.processes().values().any(|p| {
                    p.name().to_string_lossy().to_lowercase() == TARGET_PROCESS.to_lowercase()
                });

                is_running.store(found, Ordering::Relaxed);

                // Sleep before next check (2 seconds)
                thread::sleep(Duration::from_secs(2));
            }
        });

        self._thread_handle = Some(handle);
    }

    /// Check if the target process is running
    pub fn is_target_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Stop the monitoring thread
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Check once if the process is running (without starting monitor thread)
    pub fn check_once() -> bool {
        let mut system = System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        system
            .processes()
            .values()
            .any(|p| p.name().to_string_lossy().to_lowercase() == TARGET_PROCESS.to_lowercase())
    }

    /// Check if the target process is the foreground window
    #[cfg(target_os = "windows")]
    pub fn is_game_active() -> bool {
        use windows::Win32::Foundation::MAX_PATH;
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return false;
            }

            let mut process_id = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));

            if process_id == 0 {
                return false;
            }

            // PROCESS_QUERY_LIMITED_INFORMATION is sufficient for QueryFullProcessImageName
            // and works for elevated processes even if we are not elevated.
            let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id);

            match process_handle {
                Ok(handle) => {
                    let mut buffer = [0u16; MAX_PATH as usize];
                    let mut size = MAX_PATH;

                    let result = QueryFullProcessImageNameW(
                        handle,
                        PROCESS_NAME_WIN32,
                        windows::core::PWSTR(buffer.as_mut_ptr()),
                        &mut size,
                    );

                    if result.is_ok() {
                        let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
                        // Extract filename from full path
                        let name = std::path::Path::new(&full_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");

                        // Simple case-insensitive check
                        let name_lower = name.to_lowercase();
                        let target_lower = TARGET_PROCESS.to_lowercase();
                        let self_process = "akef-combonavi.exe";
                        let self_lower = self_process.to_lowercase();

                        let is_match = name_lower == target_lower || name_lower == self_lower;

                        #[cfg(debug_assertions)]
                        if is_match {
                            println!("[DEBUG] Foreground match: {}", name);
                        } else {
                            // Print what we found if it's not a match, to help debugging
                            // Only print occasionally or if it changes to avoid spam?
                            // For now, let's just print it to see what's going on.
                            println!(
                                "[DEBUG] Foreground mismatch: {} (Target: {} or {})",
                                name, TARGET_PROCESS, self_process
                            );
                        }

                        return is_match;
                    } else {
                        #[cfg(debug_assertions)]
                        println!(
                            "[DEBUG] QueryFullProcessImageNameW failed for PID: {}",
                            process_id
                        );
                    }
                    false
                }
                Err(e) => {
                    #[cfg(debug_assertions)]
                    println!(
                        "[DEBUG] Failed to open process (PID: {}): {:?}",
                        process_id, e
                    );
                    false
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_game_active() -> bool {
        // Fallback for non-Windows: just check if process exists
        Self::check_once()
    }
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ProcessMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_monitor_creation() {
        let monitor = ProcessMonitor::new();
        // Initially, is_running should be false
        assert!(!monitor.is_target_running());
    }
}

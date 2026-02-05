//! Process monitoring module
//!
//! Monitors for the Endfield.exe process to control overlay visibility.

use sysinfo::System;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
                let found = system
                    .processes()
                    .values()
                    .any(|p| p.name().to_string_lossy().to_lowercase() == TARGET_PROCESS.to_lowercase());
                
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

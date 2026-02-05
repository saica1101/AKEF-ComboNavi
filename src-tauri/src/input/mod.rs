//! Global key input handler
//!
//! Handles global keyboard hooks and implements tap/hold detection logic.

use parking_lot::RwLock;
use rdev::{listen, Event, EventType, Key};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::combo::{ComboCommand, InputType, KeyIdentifier};

/// Key event types for the input handler
#[derive(Debug, Clone)]
pub enum KeyEvent {
    /// Key was pressed (tap detected or hold started)
    KeyDown(Key),
    /// Key was released
    KeyUp(Key),
    /// Hold threshold reached
    HoldComplete(Key),
    /// Tap completed (key released before hold threshold)
    TapComplete(Key),
    /// Hold progress update (key, progress 0.0-1.0)
    HoldProgress(Key, f32),
}

/// State of a pressed key
#[derive(Debug, Clone)]
struct KeyState {
    press_time: Instant,
    hold_triggered: bool,
    /// Whether this key press has been consumed by a tap command
    consumed: bool,
}

/// Input handler for combo navigation
#[derive(Clone)]
pub struct InputHandler {
    /// Map of currently pressed keys to their state
    key_states: Arc<RwLock<HashMap<Key, KeyState>>>,
    /// Current command being waited for
    current_command: Arc<RwLock<Option<ComboCommand>>>,
    /// Hold threshold duration
    hold_threshold: Duration,
    /// Event sender
    event_sender: Option<mpsc::UnboundedSender<KeyEvent>>,
}

impl InputHandler {
    /// Create a new input handler
    pub fn new() -> Self {
        Self {
            key_states: Arc::new(RwLock::new(HashMap::new())),
            current_command: Arc::new(RwLock::new(None)),
            hold_threshold: Duration::from_millis(300),
            event_sender: None,
        }
    }

    /// Create with custom hold threshold
    pub fn with_hold_threshold(mut self, threshold_ms: u64) -> Self {
        self.hold_threshold = Duration::from_millis(threshold_ms);
        self
    }

    /// Set the current command to wait for
    pub fn set_current_command(&self, command: Option<ComboCommand>) {
        let mut current = self.current_command.write();
        *current = command;
    }

    /// Get the current command
    pub fn get_current_command(&self) -> Option<ComboCommand> {
        self.current_command.read().clone()
    }

    /// Convert rdev Key to KeyIdentifier
    fn key_to_identifier(key: &Key) -> Option<KeyIdentifier> {
        match key {
            Key::Num1 | Key::Kp1 => Some(KeyIdentifier::Number(1)),
            Key::Num2 | Key::Kp2 => Some(KeyIdentifier::Number(2)),
            Key::Num3 | Key::Kp3 => Some(KeyIdentifier::Number(3)),
            Key::Num4 | Key::Kp4 => Some(KeyIdentifier::Number(4)),
            Key::Num5 | Key::Kp5 => Some(KeyIdentifier::Number(5)),
            Key::Num6 | Key::Kp6 => Some(KeyIdentifier::Number(6)),
            Key::Num7 | Key::Kp7 => Some(KeyIdentifier::Number(7)),
            Key::Num8 | Key::Kp8 => Some(KeyIdentifier::Number(8)),
            Key::Num9 | Key::Kp9 => Some(KeyIdentifier::Number(9)),
            Key::KeyE => Some(KeyIdentifier::Chain),
            // Map Mouse Left (sentinel) to HeavyAttack (L)
            Key::Unknown(1) => Some(KeyIdentifier::HeavyAttack),
            _ => None,
        }
    }

    /// Check if the given key matches the current command
    fn matches_current_command(&self, key: &Key) -> bool {
        let current = self.current_command.read();
        if let Some(ref cmd) = *current {
            if let Some(key_id) = Self::key_to_identifier(key) {
                return cmd.key == key_id;
            }
        }
        false
    }

    /// Check if current command requires hold
    fn current_command_requires_hold(&self) -> bool {
        let current = self.current_command.read();
        if let Some(ref cmd) = *current {
            matches!(cmd.input_type, InputType::Hold { .. })
        } else {
            false
        }
    }

    /// Handle key press event
    pub fn on_key_press(&self, key: Key) -> Option<KeyEvent> {
        // Record press time
        {
            let mut states = self.key_states.write();
            if states.contains_key(&key) {
                return None;
            }
            states.insert(
                key,
                KeyState {
                    press_time: Instant::now(),
                    hold_triggered: false,
                    consumed: false,
                },
            );
        }

        // For tap commands, check immediately
        if self.matches_current_command(&key) && !self.current_command_requires_hold() {
            // Mark as consumed so release doesn't trigger logic
            if let Some(mut states) = self.key_states.try_write() {
                if let Some(state) = states.get_mut(&key) {
                    state.consumed = true;
                }
            }
            return Some(KeyEvent::TapComplete(key));
        }

        Some(KeyEvent::KeyDown(key))
    }

    /// Handle key release event
    pub fn on_key_release(&self, key: Key) -> Option<KeyEvent> {
        let state = {
            let mut states = self.key_states.write();
            states.remove(&key)
        };

        if let Some(state) = state {
            // If already consumed by tap, do nothing
            if state.consumed {
                return Some(KeyEvent::KeyUp(key));
            }

            // Only consider hold completion if it wasn't already triggered
            if !state.hold_triggered {
                let duration = state.press_time.elapsed();

                if self.matches_current_command(&key) && self.current_command_requires_hold() {
                    if duration >= self.hold_threshold {
                        return Some(KeyEvent::HoldComplete(key));
                    }
                    // Key released too early - hold not complete
                }
            }
        }

        Some(KeyEvent::KeyUp(key))
    }

    /// Check if any pressed key has reached hold threshold
    pub fn check_hold_complete(&self) -> Option<Key> {
        let mut states = self.key_states.write();

        for (key, state) in states.iter_mut() {
            // Check threshold only if not consumed and not triggered
            if !state.consumed
                && !state.hold_triggered
                && state.press_time.elapsed() >= self.hold_threshold
            {
                if self.matches_current_command(key) && self.current_command_requires_hold() {
                    state.hold_triggered = true;
                    return Some(*key);
                }
            }
        }

        None
    }

    /// Create event channel
    pub fn create_event_channel() -> (
        mpsc::UnboundedSender<KeyEvent>,
        mpsc::UnboundedReceiver<KeyEvent>,
    ) {
        mpsc::unbounded_channel()
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Start global key listener in a separate thread
pub fn start_global_key_listener(handler: InputHandler) -> mpsc::UnboundedReceiver<KeyEvent> {
    let (tx, rx) = mpsc::unbounded_channel();

    std::thread::spawn(move || {
        let handler = Arc::new(handler);
        let handler_clone = handler.clone();

        // Spawn hold check thread
        let tx_hold = tx.clone();
        let handler_hold = handler_clone.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(50));

            // Check for progress on hold keys
            let states = handler_hold.key_states.read();
            for (key, state) in states.iter() {
                if !state.consumed && !state.hold_triggered {
                    if handler_hold.matches_current_command(key)
                        && handler_hold.current_command_requires_hold()
                    {
                        let elapsed = state.press_time.elapsed();
                        let progress = (elapsed.as_millis() as f32)
                            / (handler_hold.hold_threshold.as_millis() as f32);

                        if progress >= 1.0 {
                            // Will be handled by check_hold_complete
                        } else {
                            let _ = tx_hold.send(KeyEvent::HoldProgress(*key, progress.min(1.0)));
                        }
                    }
                }
            }
            drop(states);

            if let Some(key) = handler_hold.check_hold_complete() {
                let _ = tx_hold.send(KeyEvent::HoldComplete(key));
            }
        });

        // Main event callback
        let callback = move |event: Event| match event.event_type {
            EventType::KeyPress(key) => {
                // Always send KeyDown for hotkey processing
                let _ = tx.send(KeyEvent::KeyDown(key));

                // Also process through handler for combo detection (if not Alt)
                if !matches!(key, Key::Alt | Key::AltGr) {
                    if let Some(evt) = handler_clone.on_key_press(key) {
                        // Only send if it's a combo event (Tap/Hold complete)
                        if matches!(evt, KeyEvent::TapComplete(_)) {
                            let _ = tx.send(evt);
                        }
                    }
                }
            }
            EventType::KeyRelease(key) => {
                // Always send KeyUp
                let _ = tx.send(KeyEvent::KeyUp(key));

                // Also process through handler for combo detection (if not Alt)
                if !matches!(key, Key::Alt | Key::AltGr) {
                    if let Some(evt) = handler_clone.on_key_release(key) {
                        // Only send if it's a combo event (HoldComplete)
                        if matches!(evt, KeyEvent::HoldComplete(_)) {
                            let _ = tx.send(evt);
                        }
                    }
                }
            }
            EventType::ButtonPress(rdev::Button::Left) => {
                if let Some(evt) = handler_clone.on_key_press(Key::Unknown(1)) {
                    let _ = tx.send(evt);
                }
            }
            EventType::ButtonRelease(rdev::Button::Left) => {
                if let Some(evt) = handler_clone.on_key_release(Key::Unknown(1)) {
                    let _ = tx.send(evt);
                }
            }
            _ => {}
        };

        if let Err(e) = listen(callback) {
            eprintln!("Error listening to events: {:?}", e);
        }
    });

    rx
}

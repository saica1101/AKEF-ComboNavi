/**
 * Combo navigation store
 * Manages current command state and communicates with Rust backend
 */
import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** Command information from backend */
export interface CurrentCommandInfo {
  index: number;
  total: number;
  title: string;
  key_display: string;
  character: string;
  skill_type: string;
  memo: string;
  is_hold: boolean;
}

/** Config from backend */
export interface Config {
  language: string;
  key_bindings: {
    open_settings: string;
    toggle_overlay: string;
    normal_attack: string;
    chain_attack: string;
    operator1_skill: string;
    operator2_skill: string;
    operator3_skill: string;
    operator4_skill: string;
    heavy_attack: string;
  };
  overlay: {
    opacity: number;
    x: number;
    y: number;
    width: number;
    height: number;
  };
  last_combo_file: string | null;
}

// Current command store
export const currentCommand = writable<CurrentCommandInfo | null>(null);

// Config store
export const config = writable<Config | null>(null);

// Game running status
export const isGameRunning = writable<boolean>(false);

// Overlay visibility
export const overlayVisible = writable<boolean>(true);

// Loading state
export const isLoading = writable<boolean>(false);

// Error message
export const errorMessage = writable<string | null>(null);

// Hold progress (0.0 to 1.0)
export const holdProgress = writable<number>(0);

// Derived: Progress percentage
export const progress = derived(currentCommand, ($cmd) => {
  if (!$cmd || $cmd.total === 0) return 0;
  return (($cmd.index + 1) / $cmd.total) * 100;
});

/** Load a combo file */
export async function loadComboFile(path: string): Promise<string> {
  isLoading.set(true);
  errorMessage.set(null);
  try {
    const title = await invoke<string>('load_combo_file', { path });
    await refreshCurrentCommand();
    return title;
  } catch (e) {
    errorMessage.set(String(e));
    throw e;
  } finally {
    isLoading.set(false);
  }
}

/** Refresh current command from backend */
export async function refreshCurrentCommand(): Promise<void> {
  try {
    const cmd = await invoke<CurrentCommandInfo | null>('get_current_command');
    currentCommand.set(cmd);
  } catch (e) {
    console.error('Failed to get current command:', e);
  }
}

/** Advance to next command */
export async function advanceCommand(): Promise<void> {
  try {
    const cmd = await invoke<CurrentCommandInfo | null>('advance_command');
    currentCommand.set(cmd);
  } catch (e) {
    console.error('Failed to advance command:', e);
  }
}

/** Go to previous command */
export async function previousCommand(): Promise<void> {
  try {
    const cmd = await invoke<CurrentCommandInfo | null>('previous_command');
    currentCommand.set(cmd);
  } catch (e) {
    console.error('Failed to go to previous command:', e);
  }
}

/** Reset to first command */
export async function resetCombo(): Promise<void> {
  try {
    const cmd = await invoke<CurrentCommandInfo | null>('reset_combo');
    currentCommand.set(cmd);
  } catch (e) {
    console.error('Failed to reset combo:', e);
  }
}

/** Load config from backend */
export async function loadConfig(): Promise<void> {
  try {
    const cfg = await invoke<Config>('get_config');
    config.set(cfg);
  } catch (e) {
    console.error('Failed to load config:', e);
  }
}

/** Save config to backend */
export async function saveConfig(newConfig: Config): Promise<void> {
  try {
    await invoke('save_config', { newConfig });
    config.set(newConfig);
  } catch (e) {
    errorMessage.set(String(e));
    throw e;
  }
}

/** Check if game is running */
export async function checkGameRunning(): Promise<boolean> {
  try {
    const running = await invoke<boolean>('is_game_running');
    isGameRunning.set(running);
    return running;
  } catch (e) {
    console.error('Failed to check game status:', e);
    return false;
  }
}

/** Toggle overlay visibility */
export async function toggleOverlay(): Promise<boolean> {
  try {
    const visible = await invoke<boolean>('toggle_overlay');
    overlayVisible.set(visible);
    return visible;
  } catch (e) {
    console.error('Failed to toggle overlay:', e);
    return true;
  }
}

/** Initialize event listeners */
export async function initializeListeners(): Promise<void> {
  // Listen for game status changes
  await listen<boolean>('game-status-changed', (event) => {
    isGameRunning.set(event.payload);
  });

  // Listen for combo updates (from key input handler)
  await listen<CurrentCommandInfo>('combo-update', (event) => {
    currentCommand.set(event.payload);
    holdProgress.set(0); // Reset hold progress on new command
  });

  // Listen for hold progress
  await listen<number>('hold-progress', (event) => {
    holdProgress.set(event.payload);
  });
}

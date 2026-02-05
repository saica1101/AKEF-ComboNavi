<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    currentCommand,
    overlayVisible,
    isGameRunning,
    progress,
    refreshCurrentCommand,
    advanceCommand,
    previousCommand,
    resetCombo,
    initializeListeners,
    checkGameRunning,
    config,
    loadConfig,
    holdProgress,
  } from "$lib/stores/combo";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let mounted = false;
  let isAltPressed = false;

  $: overlayOpacity = $config ? $config.overlay.opacity : 0.7;
  // Use config opacity for background
  $: backgroundStyle = `background: rgba(0, 0, 0, ${overlayOpacity})`;

  let cleanupListeners: (() => void) | null = null;
  let gameCheckInterval: number | null = null;

  onMount(() => {
    mounted = true;

    (async () => {
      await loadConfig();
      await initializeListeners();
      await checkGameRunning();
      await refreshCurrentCommand();

      // Listen for request to open settings (from global key hook)
      const unlistenSettings = await listen("request-open-settings", () => {
        openSettings();
      });

      // Listen for opacity changes from settings window
      const unlistenOpacity = await listen<number>(
        "overlay-opacity-changed",
        (event) => {
          if ($config) {
            // Update store to reflect change immediately
            config.update((c) => {
              if (!c) return null;
              return {
                ...c,
                overlay: { ...c.overlay, opacity: event.payload },
              };
            });
          }
        },
      );

      // Listen for global Alt key changes for dragging
      const unlistenAlt = await listen<boolean>(
        "alt-status-changed",
        (event) => {
          console.log("Frontend: Alt status changed:", event.payload);
          isAltPressed = event.payload;
        },
      );

      cleanupListeners = () => {
        unlistenSettings();
        unlistenOpacity();
        unlistenAlt();
      };
    })();

    // Periodically check game status
    gameCheckInterval = setInterval(
      checkGameRunning,
      5000,
    ) as unknown as number;

    return () => {
      if (cleanupListeners) cleanupListeners();
      if (gameCheckInterval) clearInterval(gameCheckInterval);
    };
  });

  // Keyboard shortcuts (for testing)
  // Note: Alt key logic is now handled by backend (alt-status-changed)
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowRight" || event.key === " ") {
      advanceCommand();
    } else if (event.key === "ArrowLeft") {
      previousCommand();
    } else if (event.key === "r" || event.key === "R") {
      resetCombo();
    }
  }

  function handleKeyup(event: KeyboardEvent) {
    // Nothing to do here for now
  }

  // Handle window blur to reset Alt state
  // Still useful as a fallback or cleanup
  function handleBlur() {
    if (isAltPressed) {
      // isAltPressed = false; // Let backend drive this?
      // Actually, if we lose focus but backend says Alt is down (global hook), we should respect backend.
      // But if we tab out or something, Rdev might lose track?
      // Rdev is global, so it should be fine.
      // Let's rely on backend event.
    }
  }

  function openSettings() {
    invoke("open_settings_window");
  }
</script>

<svelte:window
  on:keydown={handleKeydown}
  on:keyup={handleKeyup}
  on:blur={handleBlur}
/>

<!-- Drag handle (visible when Alt is pressed) -->
{#if isAltPressed}
  <div class="drag-controls">
    <div class="drag-hint">
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path d="M9 5H11V19H9V5Z" fill="white" opacity="0.8" />
        <path d="M13 5H15V19H13V5Z" fill="white" opacity="0.8" />
      </svg>
      <span>ドラッグして移動</span>
    </div>

    <button class="settings-btn" on:click={openSettings}>
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="white"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <circle cx="12" cy="12" r="3"></circle>
        <path
          d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
        ></path>
      </svg>
      設定
    </button>
  </div>
{/if}

<main
  class="overlay-container"
  class:hidden={!$overlayVisible}
  class:draggable={isAltPressed}
  data-tauri-drag-region={isAltPressed ? true : undefined}
  style={backgroundStyle}
>
  {#if !$isGameRunning}
    <div class="waiting-message">
      <div class="pulse-dot"></div>
      <span>Endfield.exe を待機中...</span>
    </div>
  {:else if $currentCommand}
    <div class="combo-display">
      <!-- Title bar -->
      <div class="title-bar">
        <span class="title">{$currentCommand.title}</span>
        <span class="progress"
          >{$currentCommand.index + 1} / {$currentCommand.total}</span
        >
      </div>

      <!-- Progress bar -->
      <div class="progress-bar">
        <div class="progress-fill" style="width: {$progress}%"></div>
      </div>

      <!-- Current command -->
      <div class="command-info">
        <div class="key-display" class:hold={$currentCommand.is_hold}>
          {#if $currentCommand.is_hold}
            <div
              class="hold-progress-fill"
              style="height: {$holdProgress * 100}%"
            ></div>
            <span class="hold-indicator">HOLD</span>
          {/if}
          <span class="key">{$currentCommand.key_display}</span>
        </div>

        <div class="details">
          <span class="character">{$currentCommand.character}</span>
          <span class="skill-type">{$currentCommand.skill_type}</span>
          {#if $currentCommand.memo}
            <span class="memo">{$currentCommand.memo}</span>
          {/if}
        </div>
      </div>
    </div>
  {:else}
    <div class="no-file">
      <span>コンボファイルを読み込んでください</span>
      <span class="hint">[Home] キーで設定を開く</span>
      <span class="hint">Altキー+設定ボタンでも開けます</span>
    </div>
  {/if}
</main>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent !important;
    font-family: "Segoe UI", "Yu Gothic UI", sans-serif;
    overflow: hidden;
    user-select: none;
  }

  .drag-controls {
    position: fixed;
    top: 8px;
    right: 8px;
    display: flex;
    gap: 8px;
    z-index: 1000;
  }

  .drag-hint {
    padding: 6px 12px;
    background: rgba(79, 195, 247, 0.9);
    border: 1px solid rgba(79, 195, 247, 1);
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 8px;
    animation: fadeIn 0.2s ease-in;
    backdrop-filter: blur(4px);
    color: white;
    font-size: 12px;
    font-weight: 600;
    pointer-events: none;
  }

  .settings-btn {
    padding: 6px 12px;
    background: rgba(42, 58, 93, 0.9);
    border: 1px solid rgba(79, 195, 247, 1);
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
    color: white;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    animation: fadeIn 0.2s ease-in;
    backdrop-filter: blur(4px);
  }

  .settings-btn:hover {
    background: rgba(79, 195, 247, 0.3);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .overlay-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    /* Default background, overwritten by inline style */
    background: rgba(0, 0, 0, 0.7);
    border-radius: 8px;
    transition:
      opacity 0.3s ease,
      background-color 0.2s ease;
  }

  .overlay-container.draggable {
    cursor: move;
    border: 2px solid rgba(79, 195, 247, 0.5);
  }

  .overlay-container.hidden {
    opacity: 0;
    pointer-events: none;
  }

  .waiting-message {
    display: flex;
    align-items: center;
    gap: 12px;
    color: #ffa500;
    font-size: 14px;
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.9);
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    background: #ffa500;
    border-radius: 50%;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(0.8);
    }
  }

  .combo-display {
    width: 100%;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .title-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: #ccc;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.9);
  }

  .title {
    font-weight: 600;
  }

  .progress-bar {
    height: 3px;
    background: rgba(255, 255, 255, 0.2);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #4fc3f7, #00bcd4);
    transition: width 0.3s ease;
  }

  .command-info {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .key-display {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 60px;
    padding: 8px 12px;
    background: linear-gradient(135deg, #1a1a2e, #16213e);
    border: 2px solid #4fc3f7;
    border-radius: 8px;
    transition: all 0.3s ease;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.5);
    position: relative; /* For absolute positioning of progress fill */
    overflow: hidden; /* To verify fill doesn't spill out */
  }

  .hold-progress-fill {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 100%;
    background: rgba(255, 107, 107, 0.4);
    transition: height 0.05s linear;
    z-index: 0;
  }

  .key-display.hold {
    border-color: #ff6b6b;
    animation: holdGlow 1s ease-in-out infinite;
  }

  @keyframes holdGlow {
    0%,
    100% {
      box-shadow: 0 0 5px rgba(255, 107, 107, 0.5);
    }
    50% {
      box-shadow: 0 0 15px rgba(255, 107, 107, 0.8);
    }
  }

  .hold-indicator {
    font-weight: bold;
    letter-spacing: 1px;
    z-index: 1; /* Ensure text is above fill */
  }

  .key {
    font-size: 24px;
    font-weight: bold;
    color: #fff;
    text-shadow: 0 0 5px rgba(79, 195, 247, 0.5);
    z-index: 1; /* Ensure text is above fill */
  }

  .details {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .character {
    font-size: 18px;
    font-weight: 600;
    color: #fff;
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.9);
  }

  .skill-type {
    font-size: 14px;
    color: #4fc3f7;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.9);
  }

  .memo {
    font-size: 12px;
    color: #aaa;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.9);
  }

  .no-file {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: #ccc;
    font-size: 14px;
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.9);
  }

  .hint {
    font-size: 12px;
    color: #aaa;
  }
</style>

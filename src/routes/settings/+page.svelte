<script lang="ts">
  import { onMount } from "svelte";
  import {
    config,
    loadConfig,
    saveConfig,
    loadComboFile,
    toggleOverlay,
    overlayVisible,
    type Config,
  } from "$lib/stores/combo";
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";

  let activeTab = "general";
  let localConfig: Config | null = null;
  let isSaving = false;
  let saveMessage = "";

  onMount(async () => {
    await loadConfig();
    updateLocalConfig();
  });

  $: if ($config && !localConfig) {
    updateLocalConfig();
  }

  function updateLocalConfig() {
    if ($config) {
      localConfig = JSON.parse(JSON.stringify($config));
    }
  }

  async function handleLoadFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Combo Files",
            extensions: ["txt", "csv"],
          },
        ],
      });

      if (selected && typeof selected === "string") {
        const title = await loadComboFile(selected);
        saveMessage = `読み込み完了: ${title}`;
        setTimeout(() => (saveMessage = ""), 3000);

        // Auto-show: update local config if needed
        if (localConfig) {
          localConfig.last_combo_file = selected;
        }

        // Optional: Ensure overlay is visible
        overlayVisible.set(true);
        invoke("set_overlay_visible", { visible: true });
      }
    } catch (e) {
      saveMessage = `エラー: ${e}`;
    }
  }

  function handleOpacityChange() {
    if (!localConfig || !$config) return;

    // Immediately update config store for reactivity in settings UI if needed
    config.update((c) => {
      if (!c) return null;
      return {
        ...c,
        overlay: {
          ...c.overlay,
          opacity: localConfig!.overlay.opacity,
        },
      };
    });

    // Invoke backend command for real-time update
    invoke("set_overlay_opacity", {
      opacity: localConfig.overlay.opacity,
    }).catch(console.error);
  }

  async function handleToggleOverlay() {
    await toggleOverlay();
  }

  async function handleExit() {
    await invoke("app_exit");
  }

  async function handleSave() {
    if (!localConfig) return;
    isSaving = true;
    try {
      await saveConfig(localConfig);
      saveMessage = "設定を保存しました";
      setTimeout(() => (saveMessage = ""), 3000);
    } catch (e) {
      saveMessage = `エラー: ${e}`;
    } finally {
      isSaving = false;
    }
  }

  let editingKey: string | null = null;

  function startEditing(key: string) {
    editingKey = key;
  }

  function handleMouseDown(event: MouseEvent, key: string) {
    if (!editingKey || editingKey !== key) return;
    event.preventDefault();

    let keyName = "";
    switch (event.button) {
      case 0:
        keyName = "LeftClick";
        break;
      case 1:
        keyName = "MiddleClick";
        break;
      case 2:
        keyName = "RightClick";
        break;
      default:
        return; // Ignore other buttons
    }

    // Update config
    if (localConfig && localConfig.key_bindings) {
      (localConfig.key_bindings as any)[key] = keyName;
    }

    // Slight delay to prevent re-triggering edit mode if click propagation happens
    setTimeout(() => {
      editingKey = null;
    }, 50);
  }

  function handleKeyDown(event: KeyboardEvent, key: string) {
    if (!editingKey || editingKey !== key) return;

    event.preventDefault();
    // Do NOT stop propagation here, as it might interfere with global shortcuts or drag gestures if modifiers are held

    if (event.key === "Escape") {
      editingKey = null;
      return;
    }

    // Convert key to display format
    let keyName = event.key;

    // Handle special keys
    if (keyName === " ") keyName = "Space";
    if (keyName.length === 1) keyName = keyName.toUpperCase();
    if (event.ctrlKey && keyName !== "Control") keyName = "Ctrl+" + keyName;
    if (event.altKey && keyName !== "Alt") keyName = "Alt+" + keyName;
    if (event.shiftKey && keyName !== "Shift" && keyName.length > 1)
      keyName = "Shift+" + keyName;

    // Update config
    if (localConfig && localConfig.key_bindings) {
      (localConfig.key_bindings as any)[key] = keyName;
    }

    editingKey = null;
  }

  function handleBlur() {
    editingKey = null;
  }

  const tabs = [
    { id: "general", label: "全般" },
    { id: "overlay", label: "オーバーレイ" },
    { id: "keybinds", label: "キーコンフィグ" },
    { id: "about", label: "About" },
    { id: "license", label: "ライセンス" },
  ];

  const keyBindingItems = [
    { key: "open_settings", label: "設定画面呼び出し" },
    { key: "toggle_overlay", label: "オーバーレイON/OFF" },
    { key: "normal_attack", label: "通常攻撃" },
    { key: "chain_attack", label: "連携" },
    { key: "operator1_skill", label: "オペレーター1 戦技" },
    { key: "operator2_skill", label: "オペレーター2 戦技" },
    { key: "operator3_skill", label: "オペレーター3 戦技" },
    { key: "operator4_skill", label: "オペレーター4 戦技" },
  ];
</script>

<main class="settings-container">
  <header>
    <h1>AKEF ComboNavi 設定</h1>
  </header>

  <nav class="tabs">
    {#each tabs as tab}
      <button
        class="tab"
        class:active={activeTab === tab.id}
        on:click={() => (activeTab = tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </nav>

  <div class="content">
    {#if localConfig}
      {#if activeTab === "general"}
        <section class="tab-content">
          <h2>コンボファイル</h2>
          <div class="form-group">
            <label
              >現在のファイル: {localConfig.last_combo_file || "未選択"}</label
            >
            <button class="btn secondary" on:click={handleLoadFile}
              >ファイルを選択して読み込む</button
            >
          </div>

          <h2>言語設定</h2>
          <div class="form-group">
            <label for="language">言語</label>
            <select id="language" bind:value={localConfig.language}>
              <option value="Japanese">日本語</option>
              <option value="English">English</option>
              <option value="ChineseSimplified">简体中文</option>
              <option value="ChineseTraditional">繁體中文</option>
            </select>
          </div>

          <h2>アプリケーション</h2>
          <div class="form-group">
            <button class="btn danger" on:click={handleExit}
              >アプリを終了する</button
            >
          </div>
        </section>
      {:else if activeTab === "overlay"}
        <section class="tab-content">
          <h2>オーバーレイウィンドウ</h2>
          <div class="form-group">
            <div class="toggle-row">
              <span>表示状態: {$overlayVisible ? "表示中" : "非表示"}</span>
              <button
                class="btn secondary small"
                on:click={handleToggleOverlay}
              >
                {$overlayVisible ? "非表示にする" : "表示する"}
              </button>
            </div>
          </div>
          <div class="form-group">
            <label for="opacity"
              >透過率: {Math.round(localConfig.overlay.opacity * 100)}%</label
            >
            <input
              id="opacity"
              type="range"
              min="0.1"
              max="1"
              step="0.05"
              bind:value={localConfig.overlay.opacity}
              on:input={handleOpacityChange}
            />
            <p class="help-text">
              右方向に動かすと濃く、左方向に動かすと薄くなります
            </p>
          </div>
        </section>
      {:else if activeTab === "keybinds"}
        <section class="tab-content">
          <h2>キーコンフィグ</h2>
          <div class="keybind-grid">
            {#each keyBindingItems as item}
              <div class="form-group keybind-item">
                <label for={item.key}>{item.label}</label>
                <input
                  id={item.key}
                  type="text"
                  value={editingKey === item.key
                    ? ""
                    : (localConfig.key_bindings as any)[item.key]}
                  placeholder={editingKey === item.key
                    ? "キーを入力... (Escでキャンセル)"
                    : ""}
                  readonly
                  class:editing={editingKey === item.key}
                  on:click={() => startEditing(item.key)}
                  on:keydown={(e) => handleKeyDown(e, item.key)}
                  on:mousedown={(e) => handleMouseDown(e, item.key)}
                  on:blur={handleBlur}
                />
              </div>
            {/each}
          </div>
        </section>
      {:else if activeTab === "about"}
        <section class="tab-content">
          <h2>About</h2>
          <div class="about-info">
            <p><strong>AKEF ComboNavi</strong></p>
            <p>Version: 0.1.0</p>
            <p>Arknights: Endfield 向けコンボナビゲーションツール</p>
            <div class="links">
              <a href="https://github.com" target="_blank">GitHub</a>
            </div>
            <button class="btn secondary">アップデートをチェック</button>
          </div>
        </section>
      {:else if activeTab === "license"}
        <section class="tab-content">
          <h2>ライセンス</h2>
          <div class="license-text">
            <h3>ゲームリソース</h3>
            <p>
              「Arknights: Endfield」およびゲーム内で使用されるリソース
              （キャラクター名、アイコン等）はGRYPHLINE（鹰角网络）の権利です。
            </p>

            <h3>本プログラム</h3>
            <p>
              AKEF ComboNavi は GPL-3.0 ライセンスの下で公開されています。
              GPL-3.0に準拠すれば、自由に改変・再配布が可能です。
            </p>

            <h3>使用ライブラリ</h3>
            <ul>
              <li>Tauri - MIT/Apache 2.0</li>
              <li>Svelte - MIT</li>
              <li>rdev - MIT</li>
              <li>sysinfo - MIT</li>
            </ul>
          </div>
        </section>
      {/if}
    {:else}
      <p>設定を読み込み中...</p>
    {/if}
  </div>

  <footer>
    {#if saveMessage}
      <span class="save-message">{saveMessage}</span>
    {/if}
    <button class="btn primary" on:click={handleSave} disabled={isSaving}>
      {isSaving ? "保存中..." : "保存"}
    </button>
  </footer>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #1a1a2e;
    color: #fff;
    font-family: "Segoe UI", "Yu Gothic UI", sans-serif;
  }

  .settings-container {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  header {
    padding: 16px 24px;
    background: #16213e;
    border-bottom: 1px solid #2a3a5d;
  }

  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .tabs {
    display: flex;
    background: #16213e;
    border-bottom: 1px solid #2a3a5d;
    padding: 0 16px;
  }

  .tab {
    padding: 12px 20px;
    background: none;
    border: none;
    color: #888;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.2s;
    border-bottom: 2px solid transparent;
  }

  .tab:hover {
    color: #fff;
  }

  .tab.active {
    color: #4fc3f7;
    border-bottom-color: #4fc3f7;
  }

  .content {
    flex: 1;
    padding: 24px;
    overflow-y: auto;
  }

  .tab-content h2 {
    margin: 0 0 20px 0;
    font-size: 16px;
    color: #4fc3f7;
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    margin-bottom: 6px;
    font-size: 14px;
    color: #aaa;
  }

  .form-group input[type="text"],
  .form-group select {
    width: 100%;
    max-width: 300px;
    padding: 10px 12px;
    background: #0f0f23;
    border: 1px solid #2a3a5d;
    border-radius: 6px;
    color: #fff;
    font-size: 14px;
  }

  .form-group input[type="range"] {
    width: 100%;
    max-width: 300px;
  }

  .help-text {
    font-size: 12px;
    color: #666;
    margin-top: 4px;
  }

  .keybind-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
  }

  .keybind-item input {
    cursor: pointer;
    transition: all 0.2s;
  }

  .keybind-item input:hover {
    border-color: #4fc3f7;
  }

  .keybind-item input.editing {
    border-color: #4fc3f7;
    background: #1e3a5a;
    box-shadow: 0 0 0 2px rgba(79, 195, 247, 0.3);
  }

  .about-info {
    line-height: 1.8;
  }

  .about-info .links {
    margin: 16px 0;
  }

  .about-info a {
    color: #4fc3f7;
    text-decoration: none;
  }

  .license-text {
    line-height: 1.8;
  }

  .license-text h3 {
    color: #4fc3f7;
    margin: 20px 0 10px 0;
    font-size: 14px;
  }

  .license-text ul {
    padding-left: 20px;
  }

  footer {
    padding: 16px 24px;
    background: #16213e;
    border-top: 1px solid #2a3a5d;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 16px;
  }

  .save-message {
    font-size: 14px;
    color: #4caf50;
  }

  .btn {
    padding: 10px 24px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn.primary {
    background: #4fc3f7;
    color: #000;
  }

  .btn.primary:hover {
    background: #29b6f6;
  }

  .btn.secondary {
    background: #2a3a5d;
    color: #fff;
  }

  .btn.danger {
    background: #e53935;
    color: #fff;
  }

  .btn.danger:hover {
    background: #ef5350;
  }

  .btn.small {
    padding: 4px 12px;
    font-size: 12px;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 8px;
  }
</style>

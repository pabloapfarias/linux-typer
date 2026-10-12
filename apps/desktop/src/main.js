import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './style.css';

const app = document.querySelector('#app');

let currentConfig = null;
let currentStatus = null;
let lastCommandOutput = '';

app.innerHTML = `
  <section class="shell">
    <!-- Top Bar with Branding & Live Service Status -->
    <header class="top-bar">
      <div class="brand-section">
        <div class="app-icon-badge">🎙️</div>
        <div class="brand-text">
          <h1>Linux Voice Typer</h1>
          <p class="tagline">Ditado local acelerado por Whisper com auto-paste inteligente</p>
        </div>
      </div>
      <div class="top-actions">
        <div class="service-badge" id="service-pill">
          <span class="pulse-dot"></span>
          <span id="service-pill-text">Carregando...</span>
        </div>
        <button id="quick-toggle-service" class="primary" style="padding: 6px 14px; font-size: 0.82rem;">Iniciar</button>
        <button id="refresh-status" class="ghost" style="padding: 6px 12px; font-size: 0.82rem;" title="Atualizar dados">↻</button>
      </div>
    </header>

    <!-- Navigation Tabs -->
    <nav class="tabs-nav">
      <button class="tab-btn active" data-tab="tab-general">
        <span>Principal</span>
      </button>
      <button class="tab-btn" data-tab="tab-settings">
        <span>Configurações</span>
      </button>
      <button class="tab-btn" data-tab="tab-debug">
        <span>Diagnóstico & Debug</span>
        <span class="tab-badge" id="tab-log-count">0</span>
      </button>
    </nav>

    <!-- ============================================================== -->
    <!-- TAB 1: PRINCIPAL / CONTROLE DIÁRIO                             -->
    <!-- ============================================================== -->
    <div id="tab-general" class="tab-content active">
      <!-- Live Dictation Visual Status Banner / HUD -->
      <section class="dictation-hud" id="dictation-hud">
        <div class="hud-indicator">
          <span class="hud-status-dot" id="hud-status-dot"></span>
          <div class="hud-text-group">
            <span class="hud-main-title" id="hud-main-title">🎙️ Aguardando Ditado</span>
            <span class="hud-sub-desc" id="hud-sub-desc">Pressione a hotkey configurada para falar a qualquer momento</span>
          </div>
        </div>
        <div class="hud-right-group">
          <div class="hud-timer-badge" id="hud-timer-badge" style="display: none;">
            <span class="timer-dot"></span>
            <span id="hud-timer-text">00:00</span>
          </div>
          <div class="sound-wave" id="hud-wave" style="display: none;">
            <span></span><span></span><span></span><span></span><span></span><span></span><span></span>
          </div>
        </div>
      </section>

      <!-- Mode Selection Card -->
      <section class="card">
        <div class="card-title">
          <span>Modo de Colagem Ativo</span>
          <small>Escolha o destino da digitação automática</small>
        </div>
        <div class="mode-selector">
          <div id="mode-editor" class="mode-option selected">
            <div class="mode-option-header">
              <span class="mode-title">Editor & Navegador</span>
              <span class="mode-badge">Ctrl+V</span>
            </div>
            <span class="mode-shortcut">Para uso geral</span>
            <span class="mode-desc">VS Code, LibreOffice, Chrome, WhatsApp, Obsidian e caixas de texto comuns.</span>
          </div>
          <div id="mode-terminal" class="mode-option">
            <div class="mode-option-header">
              <span class="mode-title">Terminal & Shell</span>
              <span class="mode-badge">Ctrl+Shift+V</span>
            </div>
            <span class="mode-shortcut">Para consoles</span>
            <span class="mode-desc">Ghostty, GNOME Terminal, Alacritty, Kitty e emuladores de terminal.</span>
          </div>
        </div>
      </section>

      <!-- Last Transcript Card -->
      <section class="card">
        <div class="transcript-card">
          <div class="transcript-header">
            <span class="transcript-label">
              <span>●</span> Última Transcrição
            </span>
            <button id="copy-transcript" class="ghost" style="padding: 4px 10px; font-size: 0.75rem;">Copiar texto</button>
          </div>
          <p id="last-transcript" class="transcript-content">Nada transcrito ainda. Pressione seu atalho e fale no microfone.</p>
        </div>
      </section>

      <!-- Quick Stats & Environment Summary -->
      <section class="grid-4" style="margin-bottom: 20px;">
        <div class="stat-item">
          <div class="stat-label">Microfone</div>
          <div class="stat-value" id="status-mic">-</div>
        </div>
        <div class="stat-item">
          <div class="stat-label">Idioma Whisper</div>
          <div class="stat-value" id="status-language">-</div>
        </div>
        <div class="stat-item clickable" id="stat-hotkey-item" title="Clique para alterar ou gravar este atalho">
          <div class="stat-label">Atalho de Voz <span class="stat-badge-action">Gravar</span></div>
          <div class="stat-value" id="status-hotkey">-</div>
        </div>
        <div class="stat-item">
          <div class="stat-label">Backend de Teclas</div>
          <div class="stat-value" id="status-backend">-</div>
        </div>
      </section>

      <!-- Actions & Notification Card -->
      <section class="card">
        <div class="card-title">
          <span>Ações Rápidas</span>
        </div>
        <div class="actions-row">
          <button id="start-service" class="primary">Iniciar Serviço</button>
          <button id="stop-service" class="danger">Pausar Serviço</button>
          <button id="paste-test">Testar Colagem Automática</button>
          <button id="open-config" class="ghost">Ver Arquivo config.toml</button>
        </div>
        <div class="hint-banner">
          <span>💡</span>
          <div>
            <strong>Dica de uso:</strong> Ao fechar esta janela, o aplicativo continua em segundo plano na bandeja do sistema (System Tray) quando ela está disponível no seu ambiente. Use a hotkey global a qualquer momento para ditar.
          </div>
        </div>
      </section>
    </div>

    <!-- ============================================================== -->
    <!-- TAB 2: CONFIGURAÇÕES                                           -->
    <!-- ============================================================== -->
    <div id="tab-settings" class="tab-content">
      <section class="card">
        <div class="card-title">
          <span>Preferências de Voz e Teclado</span>
          <div class="actions-row">
            <button id="open-config-btn-2" class="ghost" style="padding: 6px 12px; font-size: 0.82rem;">Abrir config.toml</button>
            <button id="save-config" class="primary">Salvar Configurações</button>
          </div>
        </div>

        <form class="form-grid" id="settings-form" onsubmit="return false;">
          <!-- Atalho Global com Gravador Interativo -->
          <div class="form-group full-width hotkey-card-container">
            <div class="hotkey-header-row">
              <label for="cfg-hotkey">Atalho Global de Ditado</label>
              <div class="hotkey-actions-mini">
                <button type="button" id="record-hotkey-btn" class="hotkey-record-btn" title="Gravar atalho pressionando diretamente no teclado">
                  <span class="record-dot"></span>
                  <span id="record-btn-text">Gravar Tecla</span>
                </button>
                <button type="button" id="reset-hotkey-btn" class="ghost" style="padding: 5px 10px; font-size: 0.8rem;" title="Restaurar padrão (Control+Alt+Space)">
                  Restaurar Padrão
                </button>
              </div>
            </div>

            <div class="hotkey-display-area">
              <div class="hotkey-visual-chips" id="hotkey-visual-chips"></div>
              <input id="cfg-hotkey" type="text" placeholder="Ex: Control+Alt+Space ou F8" autocomplete="off" spellcheck="false" />
            </div>

            <div id="hotkey-recording-banner" class="hotkey-recording-banner" style="display: none;">
              <div class="record-pulse-dot"></div>
              <div class="record-banner-text">
                <span class="banner-title" id="hotkey-banner-title">Ouvindo teclado... Pressione a tecla ou combinação desejada!</span>
                <span class="banner-sub" id="hotkey-banner-sub">Exemplos: <code>F8</code>, <code>F12</code>, <code>Alt+Space</code>, <code>Control+Alt+Space</code>. Pressione <strong>ESC</strong> para cancelar.</span>
              </div>
            </div>

            <span class="field-hint">
              Clique em <strong>Gravar Tecla</strong> e aperte diretamente no seu teclado qualquer tecla (como <code>F8</code>, <code>F12</code>, teclas de macro) ou combinação (<code>Alt+Space</code>, <code>Control+Alt+Space</code>).
            </span>
          </div>

          <div class="form-group">
            <label for="cfg-language">Idioma do Whisper</label>
            <input id="cfg-language" type="text" placeholder="pt, en, es, etc." />
            <span class="field-hint">Código de idioma ISO (use 'pt' para Português).</span>
          </div>

          <div class="form-group">
            <label for="cfg-shortcut">Modo de Colagem Padrão</label>
            <select id="cfg-shortcut">
              <option value="ctrl_v">Editor / Comum (Ctrl+V)</option>
              <option value="ctrl_shift_v">Terminal (Ctrl+Shift+V)</option>
            </select>
            <span class="field-hint">Comportamento inicial ao abrir o programa.</span>
          </div>

          <div class="form-group">
            <label for="cfg-backend">Backend de Injeção de Teclas</label>
            <select id="cfg-backend">
              <option value="uinput">uinput (Nativo do Kernel Linux - Recomendado)</option>
              <option value="ydotool">ydotool</option>
              <option value="wtype">wtype (Wayland wlroots)</option>
              <option value="none">none (Apenas copiar para o clipboard)</option>
              <option value="auto">auto (Detecção automática)</option>
            </select>
            <span class="field-hint">Método utilizado para simular o atalho de colar.</span>
          </div>

          <!-- Caminhos de Modelo e Executável -->
          <div class="form-group full-width">
            <label for="cfg-model">Caminho do Modelo Whisper (.bin)</label>
            <input id="cfg-model" type="text" placeholder="/caminho/para/ggml-small.bin" />
            <span class="field-hint">Arquivo do modelo de IA compilado para Whisper.cpp.</span>
          </div>

          <div class="form-group full-width">
            <label for="cfg-whisper">Executável Whisper CLI</label>
            <input id="cfg-whisper" type="text" placeholder="/caminho/para/whisper-cli" />
            <span class="field-hint">Binário compilado de inferência de áudio local.</span>
          </div>

          <div class="form-group full-width">
            <label for="cfg-temp">Diretório de Gravações Temporárias</label>
            <input id="cfg-temp" type="text" placeholder="/tmp ou ~/.local/share/..." />
            <span class="field-hint">Onde os arquivos de áudio WAV são gravados antes da transcrição.</span>
          </div>

          <!-- Opções booleanas -->
          <div class="form-group full-width" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 12px; margin-top: 8px;">
            <label class="checkbox-card">
              <input id="cfg-auto-start" type="checkbox" />
              <div>
                <span class="check-label">Iniciar Serviço Automaticamente</span>
                <span class="check-sub">Ativa o serviço de ditado automaticamente assim que você abre o aplicativo.</span>
              </div>
            </label>

            <label class="checkbox-card">
              <input id="cfg-start-system" type="checkbox" />
              <div>
                <span class="check-label">Iniciar com o Sistema (Autostart)</span>
                <span class="check-sub">Inicia o Linux Voice Typer automaticamente ao fazer login na sua sessão Linux.</span>
              </div>
            </label>

            <label class="checkbox-card">
              <input id="cfg-restore" type="checkbox" />
              <div>
                <span class="check-label">Restaurar Clipboard Original</span>
                <span class="check-sub">Restaura o texto anterior que você havia copiado após colar o ditado.</span>
              </div>
            </label>

            <label class="checkbox-card">
              <input id="cfg-start-minimized" type="checkbox" />
              <div>
                <span class="check-label">Iniciar Minimizado na Bandeja</span>
                <span class="check-sub">Oculta a janela principal e mantém o app direto na bandeja superior ao iniciar.</span>
              </div>
            </label>

            <label class="checkbox-card">
              <input id="cfg-close-to-tray" type="checkbox" />
              <div>
                <span class="check-label">Fechar para a Bandeja</span>
                <span class="check-sub">Ao fechar a janela, mantém o app rodando no System Tray. Desative para que o fechamento encerre o aplicativo (útil no GNOME sem extensão AppIndicator).</span>
              </div>
            </label>

            <label class="checkbox-card">
              <input id="cfg-osd-enabled" type="checkbox" />
              <div>
                <span class="check-label">Mostrar Notificações (OSD)</span>
                <span class="check-sub">Exibe avisos de desktop para gravação, transcrição, erros e início/parada do serviço.</span>
              </div>
            </label>
          </div>
        </form>
      </section>
    </div>

    <!-- ============================================================== -->
    <!-- TAB 3: DIAGNÓSTICO & DEBUG                                     -->
    <!-- ============================================================== -->
    <div id="tab-debug" class="tab-content">
      <!-- Diagnostic Action Tools -->
      <section class="card">
        <div class="card-title">
          <span>Ferramentas de Diagnóstico</span>
          <small>Verificação de permissões, áudio e injeção do sistema operacional</small>
        </div>
        <div class="actions-row">
          <button id="doctor" class="primary">🩺 Executar Doctor</button>
          <button id="paste-test-debug">📋 Testar Injeção (Paste Test)</button>
          <button id="restart-service">🔄 Reiniciar Serviço</button>
          <button id="refresh-logs" class="ghost">📄 Atualizar Logs</button>
        </div>
      </section>

      <!-- Terminal Output Console -->
      <section class="card">
        <div class="terminal-window">
          <div class="terminal-header">
            <div class="terminal-title">
              <span class="dot"></span>
              <span>Saída do Diagnóstico & Comandos</span>
            </div>
            <div class="terminal-actions">
              <button id="copy-output" class="ghost">Copiar</button>
              <button id="clear-output" class="ghost">Limpar</button>
            </div>
          </div>
          <pre id="command-output">Nenhum comando executado ainda. Clique em "Executar Doctor" acima para fazer a checagem completa do sistema.</pre>
        </div>
      </section>

      <section class="grid-2">
        <!-- Technical Environment Details -->
        <article class="card">
          <div class="card-title">
            <span>Resumo Técnico</span>
          </div>
          <div style="display: flex; flex-direction: column; gap: 10px;">
            <div class="stat-item">
              <div class="stat-label">Status do Serviço</div>
              <div class="stat-value" id="status-running">-</div>
            </div>
            <div class="stat-item">
              <div class="stat-label">Arquivo do Modelo</div>
              <div class="stat-value" id="status-model">-</div>
            </div>
            <div class="stat-item">
              <div class="stat-label">Binário Whisper</div>
              <div class="stat-value" id="status-whisper">-</div>
            </div>
            <div class="stat-item">
              <div class="stat-label">Modo Atual</div>
              <div class="stat-value" id="status-mode">-</div>
            </div>
          </div>
        </article>

        <!-- Live Daemon Events -->
        <article class="card">
          <div class="card-title">
            <span>Eventos Recentes do Daemon</span>
            <small>Logs em tempo real</small>
          </div>
          <ul id="recent-events" class="events-list">
            <li>Carregando eventos...</li>
          </ul>
        </article>
      </section>
    </div>
  </section>

  <!-- Floating Toast Notification -->
  <div id="toast" class="toast">Mensagem</div>
`;

const $ = (id) => document.getElementById(id);

// ----------------------------------------------------------------------------
// Helpers & Toasts
// ----------------------------------------------------------------------------
let toastTimer = null;
function showToast(text, duration = 3000) {
  const toast = $('toast');
  if (!toast) return;
  toast.textContent = text;
  toast.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.classList.remove('show');
  }, duration);
}

function basename(path) {
  if (!path) return '-';
  const parts = path.replace(/\\/g, '/').split('/');
  return parts[parts.length - 1] || path;
}

function setOutput(text) {
  lastCommandOutput = text || '';
  $('command-output').textContent = lastCommandOutput || 'Nenhum comando executado ainda.';
}

function setBusy(button, busy) {
  if (!button) return;
  button.disabled = busy;
  button.classList.toggle('busy', busy);
}

async function call(name, args = undefined) {
  try {
    return await invoke(name, args);
  } catch (error) {
    setOutput(`[ERRO] ${String(error)}`);
    showToast(`Erro: ${String(error)}`);
    throw error;
  }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// Tab Navigation Handling
// ----------------------------------------------------------------------------
let switchTab = () => {};

function setupTabs() {
  const tabButtons = document.querySelectorAll('.tab-btn');
  const tabContents = document.querySelectorAll('.tab-content');

  switchTab = function (targetId) {
    tabButtons.forEach((btn) => {
      btn.classList.toggle('active', btn.dataset.tab === targetId);
    });
    tabContents.forEach((content) => {
      content.classList.toggle('active', content.id === targetId);
    });
    try {
      localStorage.setItem('lvt_active_tab', targetId);
    } catch {}
  };

  tabButtons.forEach((btn) => {
    btn.addEventListener('click', () => {
      switchTab(btn.dataset.tab);
    });
  });

  const savedTab = localStorage.getItem('lvt_active_tab');
  if (savedTab && $(savedTab)) {
    switchTab(savedTab);
  }
}

// ----------------------------------------------------------------------------
// Interactive Hotkey Recorder
// ----------------------------------------------------------------------------
function renderHotkeyChips(hotkeyStr) {
  const container = $('hotkey-visual-chips');
  if (!container) return;
  container.innerHTML = '';
  const trimmed = (hotkeyStr || '').trim();
  if (!trimmed) {
    container.innerHTML = '<span style="color: var(--text-dim); font-size: 0.82rem; font-style: italic;">Nenhum atalho definido (clique em Gravar Tecla)</span>';
    return;
  }

  const parts = trimmed.split('+').map((p) => p.trim()).filter(Boolean);
  parts.forEach((part, index) => {
    const lower = part.toLowerCase();
    const isModifier = ['ctrl', 'control', 'alt', 'shift', 'super', 'meta', 'win'].some((m) => lower.startsWith(m));
    let display = part;
    if (lower === 'ctrl' || lower === 'control') display = 'Ctrl';
    else if (lower === 'alt') display = 'Alt';
    else if (lower === 'shift') display = 'Shift';
    else if (lower === 'super' || lower === 'meta' || lower === 'win') display = 'Super ❖';
    else if (lower === 'space') display = 'Espaço ␣';
    else if (lower === 'enter' || lower === 'return') display = 'Enter ↵';
    else if (lower === 'escape' || lower === 'esc') display = 'Esc';
    else if (lower === 'tab') display = 'Tab ⇥';
    else if (lower === 'backspace') display = 'Backspace ⌫';

    const chip = document.createElement('span');
    chip.className = `kbd-chip ${isModifier ? 'modifier' : ''}`;
    chip.textContent = display;
    container.appendChild(chip);

    if (index < parts.length - 1) {
      const plus = document.createElement('span');
      plus.className = 'kbd-plus';
      plus.textContent = '+';
      container.appendChild(plus);
    }
  });
}

let isRecordingHotkey = false;

function mapKeyCodeToPrimary(e) {
  const code = e.code || '';
  const key = e.key || '';

  // Function keys: F1..F24
  if (/^F([1-9]|1[0-9]|2[0-4])$/i.test(code) || /^F([1-9]|1[0-9]|2[0-4])$/i.test(key)) {
    return (code.startsWith('F') ? code : key).toUpperCase();
  }

  // Alpha keys: KeyA -> A
  if (code.startsWith('Key')) {
    return code.slice(3).toUpperCase();
  }

  // Digits: Digit0 -> 0
  if (code.startsWith('Digit')) {
    return code.slice(5);
  }

  // Numpad: Numpad0, NumpadAdd, etc.
  if (code.startsWith('Numpad')) {
    return code;
  }

  // Standard special keys
  switch (code) {
    case 'Space': return 'Space';
    case 'Enter': return 'Enter';
    case 'Tab': return 'Tab';
    case 'Backspace': return 'Backspace';
    case 'Insert': return 'Insert';
    case 'Delete': return 'Delete';
    case 'Home': return 'Home';
    case 'End': return 'End';
    case 'PageUp': return 'PageUp';
    case 'PageDown': return 'PageDown';
    case 'ArrowUp': return 'Up';
    case 'ArrowDown': return 'Down';
    case 'ArrowLeft': return 'Left';
    case 'ArrowRight': return 'Right';
    case 'CapsLock': return 'CapsLock';
    case 'PrintScreen': return 'PrintScreen';
    case 'ScrollLock': return 'ScrollLock';
    case 'Pause': return 'Pause';
    case 'ContextMenu': return 'Menu';
    case 'Backquote': return 'Backtick';
    case 'Minus': return 'Minus';
    case 'Equal': return 'Equal';
    case 'BracketLeft': return 'BracketLeft';
    case 'BracketRight': return 'BracketRight';
    case 'Backslash': return 'Backslash';
    case 'Semicolon': return 'Semicolon';
    case 'Quote': return 'Quote';
    case 'Comma': return 'Comma';
    case 'Period': return 'Period';
    case 'Slash': return 'Slash';
  }

  // Fallback for single printable characters
  if (key && key.length === 1 && key !== ' ') {
    return key.toUpperCase();
  }

  return null;
}

function handleHotkeyKeyDown(e) {
  e.preventDefault();
  e.stopPropagation();

  // ESC alone cancels recording
  if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
    stopHotkeyRecording();
    showToast('Gravação de tecla cancelada.');
    return;
  }

  const isModifierKey = ['Control', 'Alt', 'Shift', 'Meta'].includes(e.key) ||
    ['ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight', 'OSLeft', 'OSRight'].includes(e.code);

  if (isModifierKey) {
    // Show live held modifiers
    const mods = [];
    if (e.ctrlKey) mods.push('Control');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');
    if (e.metaKey) mods.push('Super');
    if (mods.length) {
      const title = $('hotkey-banner-title');
      if (title) title.textContent = `Modificadores: [ ${mods.join(' + ')} ] + (pressione a tecla principal)...`;
    }
    return;
  }

  const primary = mapKeyCodeToPrimary(e);
  if (!primary) return;

  const tokens = [];
  if (e.ctrlKey) tokens.push('Control');
  if (e.altKey) tokens.push('Alt');
  if (e.shiftKey) tokens.push('Shift');
  if (e.metaKey) tokens.push('Super');
  tokens.push(primary);

  const hotkeyString = tokens.join('+');
  const input = $('cfg-hotkey');
  if (input) input.value = hotkeyString;

  renderHotkeyChips(hotkeyString);
  stopHotkeyRecording();

  // Highlight Save button so user remembers to save
  const saveBtn = $('save-config');
  if (saveBtn) {
    saveBtn.classList.add('highlight-pulse');
    setTimeout(() => saveBtn.classList.remove('highlight-pulse'), 5000);
  }

  showToast(`Atalho gravado: ${hotkeyString}! Lembre-se de clicar em "Salvar Configurações".`);
}

function handleHotkeyKeyUp(e) {
  e.preventDefault();
  e.stopPropagation();
}

function startHotkeyRecording() {
  if (isRecordingHotkey) return;
  isRecordingHotkey = true;

  const btn = $('record-hotkey-btn');
  const btnText = $('record-btn-text');
  const banner = $('hotkey-recording-banner');
  const input = $('cfg-hotkey');

  if (btn) btn.classList.add('recording');
  if (btnText) btnText.textContent = 'Gravando... (ESC p/ cancelar)';
  if (banner) {
    banner.style.display = 'flex';
    const title = $('hotkey-banner-title');
    if (title) title.textContent = 'Ouvindo teclado... Pressione sua tecla ou combinação agora!';
  }
  if (input) {
    input.readOnly = true;
    input.placeholder = 'Pressione sua tecla no teclado agora...';
  }

  window.addEventListener('keydown', handleHotkeyKeyDown, { capture: true });
  window.addEventListener('keyup', handleHotkeyKeyUp, { capture: true });
}

function stopHotkeyRecording() {
  if (!isRecordingHotkey) return;
  isRecordingHotkey = false;

  const btn = $('record-hotkey-btn');
  const btnText = $('record-btn-text');
  const banner = $('hotkey-recording-banner');
  const input = $('cfg-hotkey');

  if (btn) btn.classList.remove('recording');
  if (btnText) btnText.textContent = 'Gravar Tecla';
  if (banner) banner.style.display = 'none';
  if (input) {
    input.readOnly = false;
    input.placeholder = 'Ex: Control+Alt+Space ou F8';
  }

  window.removeEventListener('keydown', handleHotkeyKeyDown, { capture: true });
  window.removeEventListener('keyup', handleHotkeyKeyUp, { capture: true });
}

function toggleHotkeyRecording() {
  if (isRecordingHotkey) {
    stopHotkeyRecording();
    showToast('Gravação cancelada.');
  } else {
    startHotkeyRecording();
  }
}

function resetHotkeyToDefault() {
  const defaultHotkey = 'Control+Alt+Space';
  const input = $('cfg-hotkey');
  if (input) input.value = defaultHotkey;
  renderHotkeyChips(defaultHotkey);

  const saveBtn = $('save-config');
  if (saveBtn) {
    saveBtn.classList.add('highlight-pulse');
    setTimeout(() => saveBtn.classList.remove('highlight-pulse'), 5000);
  }

  showToast('Atalho restaurado para o padrão (Control+Alt+Space). Clique em Salvar Configurações.');
}

function setupHotkeyRecorder() {
  $('record-hotkey-btn')?.addEventListener('click', toggleHotkeyRecording);
  $('reset-hotkey-btn')?.addEventListener('click', resetHotkeyToDefault);
  $('cfg-hotkey')?.addEventListener('input', (e) => renderHotkeyChips(e.target.value));

  $('stat-hotkey-item')?.addEventListener('click', () => {
    switchTab('tab-settings');
    $('cfg-hotkey')?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    setTimeout(startHotkeyRecording, 200);
  });
}

// ----------------------------------------------------------------------------
// Live Dictation HUD & State Handling
// ----------------------------------------------------------------------------
let hudDoneTimeout = null;
let activeDictationState = 'idle';

function formatTimer(totalSecs) {
  const m = Math.floor((totalSecs || 0) / 60);
  const s = (totalSecs || 0) % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function updateDictationHUD(state, transcript, level, elapsedSecs) {
  const s = (state || '').toLowerCase();
  activeDictationState = s;

  const hud = $('dictation-hud');
  const title = $('hud-main-title');
  const desc = $('hud-sub-desc');
  const wave = $('hud-wave');
  const timerBadge = $('hud-timer-badge');
  const timerText = $('hud-timer-text');

  if (!hud || !title || !desc) return;

  clearTimeout(hudDoneTimeout);
  const formattedTime = formatTimer(elapsedSecs || 0);

  if (s === 'recording') {
    hud.className = 'dictation-hud state-recording';
    title.textContent = `🔴 ESCUTANDO / GRAVANDO...`;
    desc.textContent = 'Fale agora no microfone. Solte a tecla ou pressione novamente para transcrever.';

    if (timerBadge) {
      timerBadge.style.display = 'inline-flex';
      timerBadge.className = 'hud-timer-badge recording';
      if (timerText) timerText.textContent = formattedTime;
    }

    if (wave) {
      wave.style.display = 'flex';
      // Real-time audio waveform driven by actual microphone volume
      const spans = wave.querySelectorAll('span');
      const lvl = typeof level === 'number' ? level : 0.2;
      const multipliers = [0.4, 0.7, 1.1, 1.4, 1.2, 0.8, 0.5];
      spans.forEach((span, i) => {
        const mult = multipliers[i] || 1.0;
        const h = Math.min(28, Math.max(5, Math.round(5 + lvl * mult * 23)));
        span.style.height = `${h}px`;
        span.style.opacity = `${Math.min(1, Math.max(0.4, 0.4 + lvl * 0.6))}`;
      });
    }
  } else if (s === 'transcribing') {
    hud.className = 'dictation-hud state-transcribing';
    title.textContent = `⏳ TRANSCREVENDO COM WHISPER (${formattedTime})`;
    desc.textContent = 'Processando áudio localmente e colando o texto no seu app...';

    if (timerBadge) {
      timerBadge.style.display = 'inline-flex';
      timerBadge.className = 'hud-timer-badge transcribing';
      if (timerText) timerText.textContent = formattedTime;
    }
    if (wave) wave.style.display = 'none';
  } else if (s === 'idle' && transcript && transcript.trim()) {
    hud.className = 'dictation-hud state-done';
    title.textContent = `✓ DITADO CONCLUÍDO E COLADO! (${formattedTime})`;
    const preview = transcript.length > 55 ? `${transcript.slice(0, 55)}...` : transcript;
    desc.textContent = `Texto inserido: "${preview}"`;
    if (timerBadge) timerBadge.style.display = 'none';
    if (wave) wave.style.display = 'none';

    const transcriptEl = $('last-transcript');
    if (transcriptEl) transcriptEl.textContent = transcript;

    hudDoneTimeout = setTimeout(() => {
      resetHUDToIdle();
    }, 4500);
  } else {
    resetHUDToIdle();
  }
}

function resetHUDToIdle() {
  activeDictationState = 'idle';
  const hud = $('dictation-hud');
  const title = $('hud-main-title');
  const desc = $('hud-sub-desc');
  const wave = $('hud-wave');
  const timerBadge = $('hud-timer-badge');
  if (!hud || !title || !desc) return;

  hud.className = 'dictation-hud';
  const hotkey = currentStatus?.hotkey || currentConfig?.hotkey || 'Control+Alt+Space';
  title.textContent = '🎙️ Aguardando Ditado';
  desc.textContent = `Pressione seu atalho [ ${hotkey} ] para falar a qualquer momento`;
  if (wave) wave.style.display = 'none';
  if (timerBadge) timerBadge.style.display = 'none';
}

// ----------------------------------------------------------------------------
// State Rendering
// ----------------------------------------------------------------------------
function renderStatus(status) {
  currentStatus = status;

  // Header pill & toggle button
  const pill = $('service-pill');
  const pillText = $('service-pill-text');
  const quickToggle = $('quick-toggle-service');

  if (status.running) {
    pill.classList.add('running');
    pillText.textContent = 'Serviço Ativo';
    quickToggle.textContent = 'Pausar';
    quickToggle.className = 'danger';
  } else {
    pill.classList.remove('running');
    pillText.textContent = 'Serviço Parado';
    quickToggle.textContent = 'Iniciar';
    quickToggle.className = 'primary';
  }

  // Update Live HUD only if not currently actively tracking a recording/transcribing session
  const statusState = (status.state || '').toLowerCase();
  if (statusState && statusState !== 'idle') {
    if (activeDictationState === 'idle') {
      updateDictationHUD(statusState, status.last_transcript, 0, 0);
    }
  } else if (activeDictationState === 'idle' && !hudDoneTimeout) {
    resetHUDToIdle();
  }

  // Stats
  if ($('status-running')) $('status-running').textContent = status.running ? 'Ativo (Rodando)' : 'Parado';
  if ($('status-hotkey')) $('status-hotkey').textContent = status.hotkey || '-';
  if ($('status-mode')) $('status-mode').textContent = status.paste_mode || '-';
  if ($('status-backend')) $('status-backend').textContent = status.paste_backend || '-';
  if ($('status-language')) $('status-language').textContent = (status.language || 'pt').toUpperCase();
  if ($('status-mic')) $('status-mic').textContent = status.microphone || 'Padrão do Sistema';
  if ($('status-model')) $('status-model').textContent = basename(status.model_path);
  if ($('status-whisper')) $('status-whisper').textContent = basename(status.whisper_bin);

  // Transcript
  const transcriptEl = $('last-transcript');
  if (transcriptEl) {
    transcriptEl.textContent = status.last_transcript || 'Nada transcrito ainda. Pressione seu atalho e fale no microfone.';
  }

  // Events & Mode
  renderEvents(status.recent_events || []);
  updateModeButtons(status.paste_mode);
}

function renderConfig(config) {
  currentConfig = config;
  $('cfg-hotkey').value = config.hotkey || '';
  renderHotkeyChips(config.hotkey || '');
  $('cfg-language').value = config.language || 'pt';
  $('cfg-backend').value = config.paste_backend || 'uinput';
  $('cfg-shortcut').value = config.paste_shortcut || 'ctrl_v';
  $('cfg-model').value = config.model_path || '';
  $('cfg-whisper').value = config.whisper_bin || '';
  $('cfg-temp').value = config.temp_dir || '';
  $('cfg-auto-start').checked = config.auto_start_service !== false;
  $('cfg-restore').checked = Boolean(config.restore_clipboard);
  $('cfg-start-minimized').checked = Boolean(config.start_minimized);
  $('cfg-start-system').checked = Boolean(config.start_with_system);
  $('cfg-close-to-tray').checked = config.close_to_tray !== false;
  $('cfg-osd-enabled').checked = config.osd_enabled !== false;
}

function collectConfig() {
  const base = currentConfig || {};
  return {
    ...base,
    hotkey: $('cfg-hotkey').value.trim(),
    language: $('cfg-language').value.trim() || 'pt',
    model_path: $('cfg-model').value.trim(),
    whisper_bin: $('cfg-whisper').value.trim(),
    insert_mode: base.insert_mode || 'clipboard',
    restore_clipboard: $('cfg-restore').checked,
    auto_paste: base.auto_paste !== false,
    paste_backend: $('cfg-backend').value,
    paste_shortcut: $('cfg-shortcut').value,
    trim_text: base.trim_text !== false,
    temp_dir: $('cfg-temp').value.trim(),
    auto_start_service: $('cfg-auto-start').checked,
    start_minimized: $('cfg-start-minimized').checked,
    start_with_system: $('cfg-start-system').checked,
    close_to_tray: $('cfg-close-to-tray').checked,
    osd_enabled: $('cfg-osd-enabled').checked,
  };
}

function renderEvents(events) {
  const list = $('recent-events');
  if (!list) return;

  const countBadge = $('tab-log-count');
  if (countBadge) countBadge.textContent = String(events.length || 0);

  list.innerHTML = '';
  if (!events.length) {
    const item = document.createElement('li');
    item.textContent = 'Nenhum evento recente registrado pelo daemon.';
    list.appendChild(item);
    return;
  }

  for (const event of events.slice().reverse()) {
    const item = document.createElement('li');
    item.textContent = event;
    list.appendChild(item);
  }
}

function updateModeButtons(mode) {
  const isEditor = mode === 'Editor' || mode === 'ctrl_v';
  $('mode-editor').classList.toggle('selected', isEditor);
  $('mode-terminal').classList.toggle('selected', !isEditor);
}

// ----------------------------------------------------------------------------
// API Interactions
// ----------------------------------------------------------------------------
async function refreshStatus() {
  try {
    renderStatus(await call('get_status'));
  } catch {}
}

async function refreshConfig() {
  try {
    renderConfig(await call('get_config'));
  } catch {}
}

async function boot() {
  setupTabs();
  setupHotkeyRecorder();
  try {
    await listen('dictation-state', (event) => {
      const payload = event.payload || {};
      updateDictationHUD(
        payload.state,
        payload.transcript,
        payload.level,
        payload.elapsed_secs
      );
    });
  } catch (err) {
    console.warn('Tauri event listener registration failed:', err);
  }
  await refreshConfig();
  await refreshStatus();
  setInterval(refreshStatus, 1000);
}

// ----------------------------------------------------------------------------
// Event Listeners
// ----------------------------------------------------------------------------
$('refresh-status').addEventListener('click', async () => {
  await refreshStatus();
  showToast('Status atualizado.');
});

$('refresh-logs').addEventListener('click', async () => {
  const logs = await call('get_recent_logs');
  renderEvents(logs);
  showToast('Logs atualizados.');
});

async function startServiceHandler(button) {
  if (currentStatus && currentStatus.running) {
    showToast('O serviço já está rodando.');
    return;
  }
  setBusy(button, true);
  try {
    const status = await call('start_service');
    renderStatus(status);
    setOutput(status.running ? 'Serviço de ditado iniciado com sucesso.' : 'Aguardando inicialização do serviço.');
    showToast('Serviço iniciado.');
  } finally {
    setBusy(button, false);
  }
}

async function stopServiceHandler(button) {
  if (!currentStatus || !currentStatus.running) {
    showToast('O serviço já está parado.');
    return;
  }
  setBusy(button, true);
  try {
    renderStatus(await call('stop_service'));
    setOutput('Serviço de ditado pausado com sucesso.');
    showToast('Serviço pausado.');
  } finally {
    setBusy(button, false);
  }
}

async function toggleServiceHandler(button) {
  setBusy(button, true);
  try {
    if (currentStatus && currentStatus.running) {
      renderStatus(await call('stop_service'));
      setOutput('Serviço de ditado pausado com sucesso.');
      showToast('Serviço pausado.');
    } else {
      const status = await call('start_service');
      renderStatus(status);
      setOutput(status.running ? 'Serviço de ditado iniciado com sucesso.' : 'Aguardando inicialização do serviço.');
      showToast('Serviço iniciado.');
    }
  } finally {
    setBusy(button, false);
  }
}

$('quick-toggle-service').addEventListener('click', (e) => toggleServiceHandler(e.currentTarget));
$('start-service').addEventListener('click', (e) => startServiceHandler(e.currentTarget));
$('stop-service').addEventListener('click', (e) => stopServiceHandler(e.currentTarget));

$('restart-service').addEventListener('click', async (event) => {
  const button = event.currentTarget;
  setBusy(button, true);
  try {
    const status = await call('restart_service');
    renderStatus(status);
    setOutput('Serviço reiniciado com sucesso com as novas configurações.');
    showToast('Serviço reiniciado.');
  } finally {
    setBusy(button, false);
  }
});

async function runPasteTestHandler(button) {
  setBusy(button, true);
  try {
    const result = await call('run_paste_test');
    setOutput(result.message);
    showToast(result.ok ? 'Teste de colagem concluído!' : 'Teste de colagem reportou falha.');
  } finally {
    setBusy(button, false);
  }
}

$('paste-test').addEventListener('click', (e) => runPasteTestHandler(e.currentTarget));
$('paste-test-debug').addEventListener('click', (e) => runPasteTestHandler(e.currentTarget));

$('doctor').addEventListener('click', async (event) => {
  const button = event.currentTarget;
  setBusy(button, true);
  try {
    const result = await call('run_doctor');
    setOutput(result.message);
    showToast(result.ok ? 'Diagnóstico Doctor: Tudo OK!' : 'Doctor detectou avisos ou erros.');
  } finally {
    setBusy(button, false);
  }
});

async function openConfigFile() {
  await call('open_config_file');
  setOutput('Arquivo config.toml aberto no editor padrão.');
  showToast('Arquivo config.toml aberto.');
}

$('open-config').addEventListener('click', openConfigFile);
$('open-config-btn-2').addEventListener('click', openConfigFile);

$('mode-editor').addEventListener('click', async () => {
  renderStatus(await call('set_mode_editor'));
  await refreshConfig();
  setOutput('Modo alterado para Editor / Navegador (Ctrl+V).');
  showToast('Modo Editor (Ctrl+V) ativado.');
});

$('mode-terminal').addEventListener('click', async () => {
  renderStatus(await call('set_mode_terminal'));
  await refreshConfig();
  setOutput('Modo alterado para Terminal / Console (Ctrl+Shift+V).');
  showToast('Modo Terminal (Ctrl+Shift+V) ativado.');
});

$('save-config').addEventListener('click', async (event) => {
  const button = event.currentTarget;
  setBusy(button, true);
  try {
    const config = collectConfig();
    const status = await call('save_config', { config });
    renderStatus(status);
    await refreshConfig();
    setOutput('Configurações salvas no disco.');
    showToast('Configurações salvas com sucesso!');
  } finally {
    setBusy(button, false);
  }
});

$('copy-transcript').addEventListener('click', () => {
  const text = $('last-transcript').textContent;
  if (text && !text.startsWith('Nada transcrito')) {
    navigator.clipboard.writeText(text);
    showToast('Transcrição copiada para o clipboard!');
  } else {
    showToast('Nenhum texto para copiar.');
  }
});

$('copy-output').addEventListener('click', () => {
  const text = $('command-output').textContent;
  if (text) {
    navigator.clipboard.writeText(text);
    showToast('Saída do terminal copiada!');
  }
});

$('clear-output').addEventListener('click', () => {
  setOutput('Terminal limpo.');
  showToast('Saída limpa.');
});

boot().catch((error) => setOutput(String(error)));

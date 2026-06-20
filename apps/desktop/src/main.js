import { invoke } from '@tauri-apps/api/core';
import './style.css';

const app = document.querySelector('#app');

let currentConfig = null;
let currentStatus = null;
let lastCommandOutput = '';

app.innerHTML = `
  <section class="shell">
    <header class="hero">
      <div>
        <p class="eyebrow">Linux-first local dictation</p>
        <h1>Linux Voice Typer</h1>
        <p class="subtitle">Ditado local com Whisper, clipboard e auto-paste para editor ou terminal.</p>
      </div>
      <div class="status-pill" id="service-pill">Carregando...</div>
    </header>

    <section class="grid">
      <article class="card span-7">
        <div class="card-header">
          <h2>Status</h2>
          <button id="refresh-status" class="ghost">Atualizar</button>
        </div>
        <div class="status-grid">
          <div><span>Serviço</span><strong id="status-running">-</strong></div>
          <div><span>Hotkey</span><strong id="status-hotkey">-</strong></div>
          <div><span>Modo</span><strong id="status-mode">-</strong></div>
          <div><span>Backend</span><strong id="status-backend">-</strong></div>
          <div><span>Idioma</span><strong id="status-language">-</strong></div>
          <div><span>Microfone</span><strong id="status-mic">-</strong></div>
        </div>
        <div class="transcript-box">
          <span>Última transcrição</span>
          <p id="last-transcript">Nada transcrito ainda.</p>
        </div>
      </article>

      <article class="card span-5">
        <h2>Ações</h2>
        <div class="actions">
          <button id="start-service" class="primary">Iniciar</button>
          <button id="stop-service">Parar</button>
          <button id="paste-test">Testar paste</button>
          <button id="doctor">Rodar doctor</button>
          <button id="open-config">Abrir config</button>
        </div>
        <p class="hint">Fechar a janela esconde o app no tray. Use “Sair” no tray para encerrar.</p>
      </article>

      <article class="card span-6">
        <h2>Modo de colagem</h2>
        <div class="segmented">
          <button id="mode-editor">Editor/Input comum<br><small>Ctrl+V</small></button>
          <button id="mode-terminal">Terminal<br><small>Ctrl+Shift+V</small></button>
        </div>
      </article>

      <article class="card span-6">
        <h2>Resumo técnico</h2>
        <div class="compact-list">
          <div><span>Modelo</span><strong id="status-model">-</strong></div>
          <div><span>Whisper CLI</span><strong id="status-whisper">-</strong></div>
        </div>
      </article>

      <article class="card span-12">
        <div class="card-header">
          <h2>Configurações</h2>
          <button id="save-config" class="primary">Salvar configurações</button>
        </div>
        <form class="settings" id="settings-form">
          <label>Hotkey<input id="cfg-hotkey" type="text" /></label>
          <label>Idioma<input id="cfg-language" type="text" /></label>
          <label>Backend de paste
            <select id="cfg-backend">
              <option value="uinput">uinput</option>
              <option value="ydotool">ydotool</option>
              <option value="wtype">wtype</option>
              <option value="none">none</option>
              <option value="auto">auto</option>
            </select>
          </label>
          <label>Modo de colagem
            <select id="cfg-shortcut">
              <option value="ctrl_v">Editor/Input comum - Ctrl+V</option>
              <option value="ctrl_shift_v">Terminal - Ctrl+Shift+V</option>
            </select>
          </label>
          <label class="wide">Modelo Whisper<input id="cfg-model" type="text" /></label>
          <label class="wide">Whisper CLI<input id="cfg-whisper" type="text" /></label>
          <label class="wide">Diretório temporário<input id="cfg-temp" type="text" /></label>
          <label class="check"><input id="cfg-restore" type="checkbox" /> Restaurar clipboard</label>
          <label class="check"><input id="cfg-start-minimized" type="checkbox" /> Iniciar minimizado</label>
          <label class="check disabled"><input id="cfg-start-system" type="checkbox" disabled /> Iniciar com o sistema (TODO)</label>
        </form>
      </article>

      <article class="card span-12">
        <div class="card-header">
          <h2>Eventos recentes</h2>
          <button id="refresh-logs" class="ghost">Atualizar logs</button>
        </div>
        <ul id="recent-events" class="events"></ul>
      </article>

      <article class="card span-12">
        <h2>Resultado</h2>
        <pre id="command-output">Nenhum comando executado ainda.</pre>
      </article>
    </section>
  </section>
`;

const $ = (id) => document.getElementById(id);

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
  button.disabled = busy;
  button.classList.toggle('busy', busy);
}

async function call(name, args = undefined) {
  try {
    return await invoke(name, args);
  } catch (error) {
    setOutput(String(error));
    throw error;
  }
}

function renderStatus(status) {
  currentStatus = status;
  $('service-pill').textContent = status.running ? 'Serviço rodando' : 'Serviço parado';
  $('service-pill').classList.toggle('running', status.running);
  $('status-running').textContent = status.running ? 'Rodando' : 'Parado';
  $('status-hotkey').textContent = status.hotkey;
  $('status-mode').textContent = status.paste_mode;
  $('status-backend').textContent = status.paste_backend;
  $('status-language').textContent = status.language;
  $('status-mic').textContent = status.microphone;
  $('status-model').textContent = basename(status.model_path);
  $('status-whisper').textContent = basename(status.whisper_bin);
  $('last-transcript').textContent = status.last_transcript || 'Nada transcrito ainda.';
  renderEvents(status.recent_events || []);
  updateModeButtons(status.paste_mode);
}

function renderConfig(config) {
  currentConfig = config;
  $('cfg-hotkey').value = config.hotkey || '';
  $('cfg-language').value = config.language || 'pt';
  $('cfg-backend').value = config.paste_backend || 'uinput';
  $('cfg-shortcut').value = config.paste_shortcut || 'ctrl_v';
  $('cfg-model').value = config.model_path || '';
  $('cfg-whisper').value = config.whisper_bin || '';
  $('cfg-temp').value = config.temp_dir || '';
  $('cfg-restore').checked = Boolean(config.restore_clipboard);
  $('cfg-start-minimized').checked = Boolean(config.start_minimized);
  $('cfg-start-system').checked = Boolean(config.start_with_system);
}

function collectConfig() {
  return {
    ...(currentConfig || {}),
    hotkey: $('cfg-hotkey').value.trim(),
    language: $('cfg-language').value.trim() || 'pt',
    model_path: $('cfg-model').value.trim(),
    whisper_bin: $('cfg-whisper').value.trim(),
    insert_mode: 'clipboard',
    restore_clipboard: $('cfg-restore').checked,
    auto_paste: true,
    paste_backend: $('cfg-backend').value,
    paste_shortcut: $('cfg-shortcut').value,
    trim_text: true,
    temp_dir: $('cfg-temp').value.trim(),
    start_minimized: $('cfg-start-minimized').checked,
    start_with_system: false,
  };
}

function renderEvents(events) {
  const list = $('recent-events');
  list.innerHTML = '';
  if (!events.length) {
    const item = document.createElement('li');
    item.textContent = 'Nenhum evento recente.';
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
  $('mode-editor').classList.toggle('selected', mode === 'Editor');
  $('mode-terminal').classList.toggle('selected', mode === 'Terminal');
}

async function refreshStatus() {
  renderStatus(await call('get_status'));
}

async function refreshConfig() {
  renderConfig(await call('get_config'));
}

async function boot() {
  await refreshConfig();
  await refreshStatus();
  setInterval(refreshStatus, 2500);
}

$('refresh-status').addEventListener('click', refreshStatus);
$('refresh-logs').addEventListener('click', async () => renderEvents(await call('get_recent_logs')));

$('start-service').addEventListener('click', async (event) => {
  setBusy(event.currentTarget, true);
  try {
    const status = await call('start_service');
    renderStatus(status);
    if (status.running) {
      const alreadyRunning = (status.recent_events || []).some(e => e.includes('já estava rodando'));
      setOutput(alreadyRunning ? 'Serviço já está rodando.' : 'Serviço iniciado.');
    }
  } finally {
    setBusy(event.currentTarget, false);
  }
});

$('stop-service').addEventListener('click', async (event) => {
  setBusy(event.currentTarget, true);
  try {
    renderStatus(await call('stop_service'));
    setOutput('Serviço parado.');
  } finally {
    setBusy(event.currentTarget, false);
  }
});

$('paste-test').addEventListener('click', async (event) => {
  setBusy(event.currentTarget, true);
  try {
    const result = await call('run_paste_test');
    setOutput(result.message);
  } finally {
    setBusy(event.currentTarget, false);
  }
});

$('doctor').addEventListener('click', async (event) => {
  setBusy(event.currentTarget, true);
  try {
    const result = await call('run_doctor');
    setOutput(result.message);
  } finally {
    setBusy(event.currentTarget, false);
  }
});

$('open-config').addEventListener('click', async () => {
  await call('open_config_file');
  setOutput('Arquivo de configuração aberto.');
});

$('mode-editor').addEventListener('click', async () => {
  renderStatus(await call('set_mode_editor'));
  await refreshConfig();
  setOutput('Modo Editor selecionado.');
});

$('mode-terminal').addEventListener('click', async () => {
  renderStatus(await call('set_mode_terminal'));
  await refreshConfig();
  setOutput('Modo Terminal selecionado.');
});

$('save-config').addEventListener('click', async () => {
  const config = collectConfig();
  const status = await call('save_config', { config });
  renderStatus(status);
  await refreshConfig();
  const events = status.recent_events || [];
  const lastEvent = events[events.length - 1] || '';
  if (lastEvent.includes('Reinicie')) {
    setOutput('Configuração salva. Reinicie o serviço para aplicar as alterações.');
  } else {
    setOutput('Configuração salva.');
  }
});

boot().catch((error) => setOutput(String(error)));

import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import './overlay.css';

const root = document.getElementById('overlay');
const title = document.getElementById('ov-title');
const timer = document.getElementById('ov-timer');
const wave = document.getElementById('ov-wave');

let lastSaved = { x: null, y: null };
let saveTimer = null;

function formatTimer(totalSecs) {
  const m = Math.floor((totalSecs || 0) / 60);
  const s = (totalSecs || 0) % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function render(state, level, elapsedSecs) {
  const s = (state || '').toLowerCase();
  const formatted = formatTimer(elapsedSecs || 0);

  if (s === 'recording') {
    root.className = 'overlay recording';
    title.textContent = 'Gravando';
    timer.textContent = formatted;
    wave.style.display = 'flex';

    // Barras reagem ao volume real do microfone (RMS)
    const lvl = typeof level === 'number' ? level : 0.2;
    const multipliers = [0.5, 0.9, 1.3, 1.0, 0.6];
    wave.querySelectorAll('span').forEach((span, i) => {
      const mult = multipliers[i] || 1.0;
      const h = Math.min(22, Math.max(4, Math.round(4 + lvl * mult * 18)));
      span.style.height = `${h}px`;
    });
  } else if (s === 'transcribing') {
    root.className = 'overlay transcribing';
    title.textContent = 'Transcrevendo';
    timer.textContent = formatted;
    wave.style.display = 'none';
  } else {
    root.className = 'overlay';
    title.textContent = 'Aguardando';
    timer.textContent = '--:--';
    wave.style.display = 'none';
  }
}

async function savePosition(position) {
  if (lastSaved.x === position.x && lastSaved.y === position.y) return;
  lastSaved = { x: position.x, y: position.y };
  try {
    await invoke('save_overlay_position', { x: position.x, y: position.y });
  } catch (err) {
    console.warn('failed to save overlay position:', err);
  }
}

function scheduleSavePosition(position) {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => savePosition(position), 500);
}

async function boot() {
  try {
    await listen('dictation-state', (event) => {
      const payload = event.payload || {};
      render(payload.state, payload.level, payload.elapsed_secs);
    });
  } catch (err) {
    console.warn('overlay: failed to listen for dictation-state:', err);
  }

  try {
    await getCurrentWindow().onMoved(({ payload }) => {
      scheduleSavePosition({ x: payload.x, y: payload.y });
    });
  } catch (err) {
    console.warn('overlay: failed to track window moves:', err);
  }
}

boot();

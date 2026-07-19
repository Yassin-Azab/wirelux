/**
 * ProcessPanel/ProcessBar.js
 * Single animated process bar component.
 * Returns an HTMLElement.
 */

import { formatBytes, formatPercent } from '../Utilities/format.js';

// Known process → icon letter mapping
const PROCESS_ICONS = {
  chrome:        { letter: 'C', color: '#4285f4' },
  chromium:      { letter: 'C', color: '#4285f4' },
  firefox:       { letter: 'F', color: '#ff9500' },
  'firefox-esr': { letter: 'F', color: '#ff9500' },
  cursor:        { letter: 'Cs', color: '#8b5cf6' },
  electron:      { letter: 'E', color: '#2dd4bf' },
  qbittorrent:   { letter: 'Q', color: '#00c8ff' },
  discord:       { letter: 'D', color: '#5865f2' },
  steam:         { letter: 'S', color: '#1b2838' },
  codium:        { letter: 'V', color: '#007acc' },
  code:          { letter: 'V', color: '#007acc' },
  ssh:           { letter: 'Sh', color: '#22c55e' },
  ntpd:          { letter: 'N', color: '#64748b' },
  marktext:      { letter: 'M', color: '#f59e0b' },
};

function getIcon(name) {
  const key = (name || '').toLowerCase().replace(/[^a-z-]/g, '');
  return PROCESS_ICONS[key] || { letter: name ? name[0].toUpperCase() : '?', color: '#4a6080' };
}

// Color band for bar gradient — based on percentage
function barColor(pct) {
  if (pct > 40) return 'linear-gradient(90deg, #00d4ff, #00ff88)';
  if (pct > 20) return 'linear-gradient(90deg, #00c8f0, #00e070)';
  return 'linear-gradient(90deg, #0080cc, #00b060)';
}

/**
 * Create an animated process bar element.
 * @param {{ name: string, total_bytes: number, percentage: number }} proc
 * @param {boolean} isOther  Whether this is the "Other Processes" aggregate row.
 * @returns {HTMLElement}
 */
export function createProcessBar(proc, isOther = false) {
  const { name, total_bytes, percentage } = proc;
  const icon = getIcon(name);

  const el = document.createElement('div');
  el.className = `proc-row${isOther ? ' proc-other' : ''}`;
  el.dataset.name = name;

  el.innerHTML = `
    <div class="proc-icon" style="background: ${icon.color}22; color: ${icon.color}">
      ${icon.letter}
    </div>
    <div class="proc-info">
      <div class="proc-top">
        <span class="proc-name">${escHtml(name)}</span>
        <span class="proc-bytes">${formatBytes(total_bytes)}</span>
      </div>
      <div class="proc-bar-track">
        <div class="proc-bar-fill" style="width: 0%; background: ${barColor(percentage)}"></div>
      </div>
    </div>
    <span class="proc-pct">${formatPercent(percentage)}</span>
  `;

  // Animate bar width in next frame
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      const fill = el.querySelector('.proc-bar-fill');
      if (fill) fill.style.width = `${Math.min(100, percentage)}%`;
    });
  });

  return el;
}

/**
 * Update an existing process bar element with new values.
 * @param {HTMLElement} el
 * @param {{ total_bytes: number, percentage: number }} proc
 */
export function updateProcessBar(el, proc) {
  const { total_bytes, percentage } = proc;
  const bytesEl = el.querySelector('.proc-bytes');
  const pctEl   = el.querySelector('.proc-pct');
  const fill    = el.querySelector('.proc-bar-fill');

  if (bytesEl) bytesEl.textContent = formatBytes(total_bytes);
  if (pctEl)   pctEl.textContent   = formatPercent(percentage);
  if (fill)    fill.style.width    = `${Math.min(100, percentage)}%`;
}

function escHtml(s) {
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

/**
 * Map/Tooltip/index.js
 * Floating tooltip that displays aggregated connection statistics.
 */

import { formatBytes, formatNumber } from '../../Utilities/format.js';

const el = document.getElementById('connection-tooltip');
let hideTimer = null;

// ── Styles injected once ────────────────────────────────────────────────────
const style = document.createElement('style');
style.textContent = `
  #connection-tooltip {
    background: rgba(11, 20, 34, 0.96);
    border: 1px solid rgba(0, 212, 255, 0.2);
    border-radius: 10px;
    padding: 14px 18px;
    min-width: 240px;
    max-width: 300px;
    box-shadow: 0 4px 32px rgba(0,0,0,0.7), 0 0 0 1px rgba(0,212,255,0.08);
    backdrop-filter: blur(12px);
    font-family: 'Inter', sans-serif;
    font-size: 12px;
    color: #e8f0fe;
    pointer-events: none;
    transition: opacity 0.15s ease;
  }
  .tt-header {
    font-size: 13px;
    font-weight: 600;
    color: #00d4ff;
    margin-bottom: 10px;
    padding-bottom: 8px;
    border-bottom: 1px solid rgba(0,212,255,0.12);
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .tt-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: #00d4ff;
    box-shadow: 0 0 6px #00d4ff;
    flex-shrink: 0;
  }
  .tt-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    padding: 3px 0;
  }
  .tt-label {
    color: #4a6080;
    font-size: 11px;
    white-space: nowrap;
  }
  .tt-value {
    color: #e8f0fe;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    text-align: right;
    flex-shrink: 0;
  }
  .tt-value.highlight {
    color: #00ff88;
  }
  .tt-divider {
    border: none;
    border-top: 1px solid rgba(0,212,255,0.08);
    margin: 7px 0;
  }
`;
document.head.appendChild(style);

/**
 * Show the tooltip near (x, y) with connection data.
 * @param {{ x: number, y: number }} position  Mouse page coordinates.
 * @param {object} data                         Aggregated connection row.
 */
export function showTooltip(position, data) {
  if (hideTimer) { clearTimeout(hideTimer); hideTimer = null; }

  el.innerHTML = `
    <div class="tt-header">
      <div class="tt-dot"></div>
      ${escapeHtml(data.country || 'Unknown')}
    </div>
    <div class="tt-row">
      <span class="tt-label">Total Traffic</span>
      <span class="tt-value highlight">${formatBytes(data.total_bytes)}</span>
    </div>
    <div class="tt-row">
      <span class="tt-label">Incoming</span>
      <span class="tt-value">${formatBytes(data.incoming_bytes)}</span>
    </div>
    <div class="tt-row">
      <span class="tt-label">Outgoing</span>
      <span class="tt-value">${formatBytes(data.outgoing_bytes)}</span>
    </div>
    <hr class="tt-divider"/>
    <div class="tt-row">
      <span class="tt-label">Packets</span>
      <span class="tt-value">${formatNumber(data.packet_count)}</span>
    </div>
    <div class="tt-row">
      <span class="tt-label">Unique Processes</span>
      <span class="tt-value">${formatNumber(data.unique_processes)}</span>
    </div>
    <div class="tt-row">
      <span class="tt-label">Top Process</span>
      <span class="tt-value highlight">${escapeHtml(data.top_process || '—')}</span>
    </div>
    <hr class="tt-divider"/>
    <div class="tt-row">
      <span class="tt-label">Protocols</span>
      <span class="tt-value">${escapeHtml(data.protocols || '—')}</span>
    </div>
    <div class="tt-row">
      <span class="tt-label">First Packet</span>
      <span class="tt-value">${escapeHtml(data.first_packet || '—')}</span>
    </div>
    <div class="tt-row">
      <span class="tt-label">Last Packet</span>
      <span class="tt-value">${escapeHtml(data.last_packet || '—')}</span>
    </div>
  `;

  positionTooltip(position.x, position.y);
  el.classList.add('visible');
}

/**
 * Move tooltip to follow mouse.
 * @param {number} x
 * @param {number} y
 */
export function moveTooltip(x, y) {
  positionTooltip(x, y);
}

/**
 * Hide the tooltip.
 * @param {number} [delay=100]
 */
export function hideTooltip(delay = 100) {
  hideTimer = setTimeout(() => {
    el.classList.remove('visible');
    hideTimer = null;
  }, delay);
}

function positionTooltip(x, y) {
  const margin = 16;
  const tw = el.offsetWidth || 260;
  const th = el.offsetHeight || 220;
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  let left = x + margin;
  let top  = y - th / 2;

  if (left + tw > vw - 8) left = x - tw - margin;
  if (top < 8)             top = 8;
  if (top + th > vh - 8)  top = vh - th - 8;

  el.style.left = `${left}px`;
  el.style.top  = `${top}px`;
}

function escapeHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

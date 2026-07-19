/**
 * ProcessPanel/index.js
 * Renders the process bandwidth statistics panel.
 *
 * Features:
 * - Auto-calculates how many rows fit in the panel height
 * - Last row is always "Other Processes" (aggregated tail)
 * - Smooth animated bar widths
 * - Reactive to resize
 */

import { createProcessBar, updateProcessBar } from './ProcessBar.js';
import { formatBytes } from '../Utilities/format.js';

const ROW_HEIGHT    = 60; // px per process row (approximate)
const HEADER_HEIGHT = 42; // section header
const MIN_ROWS      = 3;

// Styles injected once
const style = document.createElement('style');
style.textContent = `
  #process-panel-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }

  .proc-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 18px 8px;
    flex-shrink: 0;
    border-bottom: 1px solid rgba(0,212,255,0.08);
  }

  .proc-section-title {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #4a6080;
  }

  .proc-total-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: #00d4ff;
  }

  .proc-list {
    flex: 1;
    overflow: hidden;
    padding: 6px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .proc-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 18px;
    border-radius: 6px;
    margin: 0 8px;
    transition: background 0.15s ease;
    cursor: default;
    flex-shrink: 0;
  }
  .proc-row:hover {
    background: rgba(0, 212, 255, 0.04);
  }
  .proc-row.proc-other {
    border-top: 1px solid rgba(0, 212, 255, 0.07);
    margin-top: 4px;
    padding-top: 10px;
    opacity: 0.7;
  }

  .proc-icon {
    width: 30px;
    height: 30px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    flex-shrink: 0;
    letter-spacing: -0.5px;
  }

  .proc-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .proc-top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 6px;
  }

  .proc-name {
    font-size: 12px;
    font-weight: 500;
    color: #e8f0fe;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 140px;
  }

  .proc-bytes {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: #8ba3cc;
    flex-shrink: 0;
  }

  .proc-bar-track {
    height: 4px;
    background: rgba(255,255,255,0.06);
    border-radius: 2px;
    overflow: hidden;
  }

  .proc-bar-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .proc-pct {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: #4a6080;
    width: 36px;
    text-align: right;
    flex-shrink: 0;
  }

  .proc-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: #4a6080;
    padding: 24px;
    text-align: center;
  }
`;
document.head.appendChild(style);

// ── State ─────────────────────────────────────────────────────────────────────
let _container = null;
let _listEl    = null;
let _totalEl   = null;
let _lastData  = [];
let _maxRows   = 5;

/**
 * Mount the process panel into `container`.
 * @param {HTMLElement} container  #process-panel-container
 */
export function initProcessPanel(container) {
  _container = container;

  container.innerHTML = `
    <div class="proc-section-header">
      <span class="proc-section-title">Process Traffic</span>
      <span class="proc-total-label" id="proc-total-label">—</span>
    </div>
    <div class="proc-list" id="proc-list"></div>
  `;

  _listEl  = container.querySelector('#proc-list');
  _totalEl = container.querySelector('#proc-total-label');

  // Observe height changes
  const ro = new ResizeObserver(() => _recalcAndRender());
  ro.observe(container);
}

/**
 * Set / update process statistics.
 * @param {{ processes: Array, total_bytes: number }} data
 */
export function setProcessStatistics(data) {
  _lastData = data;
  _recalcAndRender();
}

// ── Internal ──────────────────────────────────────────────────────────────────

function _recalcAndRender() {
  if (!_listEl || !_lastData || !_lastData.processes) return;

  const availH = _listEl.clientHeight || _container.clientHeight - HEADER_HEIGHT;
  _maxRows = Math.max(MIN_ROWS, Math.floor(availH / ROW_HEIGHT) - 1); // -1 for "Other"

  _render(_lastData);
}

function _render({ processes, total_bytes }) {
  if (!_listEl) return;

  if (_totalEl) _totalEl.textContent = formatBytes(total_bytes);

  if (!processes || processes.length === 0) {
    _listEl.innerHTML = '<div class="proc-empty">No process data available.<br/>Apply different filters or wait for traffic.</div>';
    return;
  }

  const visible = processes.slice(0, _maxRows);
  const tail    = processes.slice(_maxRows);

  // Build "Other Processes" row
  const otherBytes = tail.reduce((s, p) => s + p.total_bytes, 0);
  const otherPct   = total_bytes > 0 ? otherBytes / total_bytes * 100 : 0;

  const toRender = [...visible];
  if (tail.length > 0) {
    toRender.push({
      name: `Other Processes (${tail.length})`,
      total_bytes: otherBytes,
      percentage: otherPct,
      _isOther: true,
    });
  }

  // Efficient DOM update — reuse existing rows where possible
  const existing = Array.from(_listEl.querySelectorAll('.proc-row'));

  toRender.forEach((proc, i) => {
    if (existing[i] && existing[i].dataset.name === proc.name) {
      // Update in place
      updateProcessBar(existing[i], proc);
    } else {
      // Replace
      const el = createProcessBar(proc, proc._isOther || false);
      if (existing[i]) {
        _listEl.replaceChild(el, existing[i]);
      } else {
        _listEl.appendChild(el);
      }
    }
  });

  // Remove excess rows
  for (let i = toRender.length; i < existing.length; i++) {
    existing[i].remove();
  }
}

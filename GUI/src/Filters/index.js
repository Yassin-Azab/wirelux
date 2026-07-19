/**
 * Filters/index.js
 * Filter toolbar — direction, protocol, time range, process search, country filter.
 *
 * Emits a `filtersChanged` CustomEvent on the document whenever any filter changes.
 * The event detail contains the current filter state object.
 */

import { debounce } from '../Utilities/format.js';

// Styles
const style = document.createElement('style');
style.textContent = `
  #filters-container {
    flex-shrink: 0;
    border-bottom: 1px solid rgba(0, 212, 255, 0.08);
  }

  .filter-section {
    padding: 10px 16px;
    border-bottom: 1px solid rgba(0, 212, 255, 0.05);
  }
  .filter-section:last-child { border-bottom: none; }

  .filter-label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #4a6080;
    margin-bottom: 6px;
    display: block;
  }

  .chip-row {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .filter-chip {
    display: inline-flex;
    align-items: center;
    padding: 3px 9px;
    border-radius: 20px;
    font-size: 11px;
    font-weight: 500;
    border: 1px solid rgba(0,212,255,0.12);
    background: transparent;
    color: #8ba3cc;
    cursor: pointer;
    transition: all 0.12s ease;
    user-select: none;
    white-space: nowrap;
    font-family: 'Inter', sans-serif;
  }
  .filter-chip:hover {
    border-color: #00d4ff;
    color: #00d4ff;
    background: rgba(0,212,255,0.07);
  }
  .filter-chip.active {
    border-color: #00d4ff;
    color: #00d4ff;
    background: rgba(0,212,255,0.12);
    box-shadow: 0 0 8px rgba(0,212,255,0.2);
  }

  .filter-search {
    width: 100%;
    background: rgba(4, 10, 20, 0.8);
    border: 1px solid rgba(0,212,255,0.1);
    border-radius: 6px;
    color: #e8f0fe;
    font-family: 'Inter', sans-serif;
    font-size: 12px;
    padding: 6px 10px;
    outline: none;
    transition: border-color 0.12s, box-shadow 0.12s;
  }
  .filter-search::placeholder { color: #4a6080; }
  .filter-search:focus {
    border-color: #00d4ff;
    box-shadow: 0 0 0 2px rgba(0,212,255,0.1);
  }

  .filter-select {
    width: 100%;
    background: rgba(4, 10, 20, 0.8);
    border: 1px solid rgba(0,212,255,0.1);
    border-radius: 6px;
    color: #e8f0fe;
    font-family: 'Inter', sans-serif;
    font-size: 12px;
    padding: 6px 28px 6px 10px;
    outline: none;
    appearance: none;
    cursor: pointer;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%234a6080'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
    transition: border-color 0.12s;
  }
  .filter-select:focus { border-color: #00d4ff; }

  .filter-row-inline {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
`;
document.head.appendChild(style);

// ── State ─────────────────────────────────────────────────────────────────────
const state = {
  direction: 'both',
  protocol:  'all',
  timeRange: 'all',
  process:   '',
  country:   '',
};

// ── Public API ─────────────────────────────────────────────────────────────────

/**
 * Mount filters into `container`.
 * @param {HTMLElement} container  #filters-container
 */
export function initFilters(container) {
  container.innerHTML = `
    <!-- Direction -->
    <div class="filter-section">
      <span class="filter-label">Direction</span>
      <div class="chip-row" id="filter-direction">
        <button class="filter-chip active" data-value="both">Both</button>
        <button class="filter-chip" data-value="incoming">Incoming</button>
        <button class="filter-chip" data-value="outgoing">Outgoing</button>
      </div>
    </div>

    <!-- Protocol -->
    <div class="filter-section">
      <span class="filter-label">Protocol</span>
      <div class="chip-row" id="filter-protocol">
        <button class="filter-chip active" data-value="all">All</button>
        <button class="filter-chip" data-value="tcp">TCP</button>
        <button class="filter-chip" data-value="udp">UDP</button>
      </div>
    </div>

    <!-- Time + Country (inline grid) -->
    <div class="filter-section">
      <div class="filter-row-inline">
        <div>
          <span class="filter-label">Time Range</span>
          <select class="filter-select" id="filter-time">
            <option value="all">All Time</option>
            <option value="10">Last 10 min</option>
            <option value="30">Last 30 min</option>
            <option value="60">Last Hour</option>
            <option value="180">Last 3 Hours</option>
            <option value="360">Last 6 Hours</option>
            <option value="720">Last 12 Hours</option>
            <option value="1440">Last 24 Hours</option>
          </select>
        </div>
        <div>
          <span class="filter-label">Country</span>
          <input class="filter-search" id="filter-country" type="text" placeholder="Filter country…" autocomplete="off" />
        </div>
      </div>
    </div>

    <!-- Process search -->
    <div class="filter-section">
      <span class="filter-label">Process Search</span>
      <input class="filter-search" id="filter-process" type="text" placeholder="Search process…" autocomplete="off" />
    </div>
  `;

  // ── Wire up events ────────────────────────────────────────────────────────
  // Direction chips
  container.querySelector('#filter-direction').addEventListener('click', e => {
    const btn = e.target.closest('.filter-chip');
    if (!btn) return;
    setActiveChip('#filter-direction', btn.dataset.value);
    state.direction = btn.dataset.value;
    emit();
  });

  // Protocol chips
  container.querySelector('#filter-protocol').addEventListener('click', e => {
    const btn = e.target.closest('.filter-chip');
    if (!btn) return;
    setActiveChip('#filter-protocol', btn.dataset.value);
    state.protocol = btn.dataset.value;
    emit();
  });

  // Time range
  container.querySelector('#filter-time').addEventListener('change', e => {
    state.timeRange = e.target.value;
    emit();
  });

  // Process search (debounced)
  container.querySelector('#filter-process').addEventListener('input',
    debounce(e => { state.process = e.target.value; emit(); }, 350)
  );

  // Country search (debounced)
  container.querySelector('#filter-country').addEventListener('input',
    debounce(e => { state.country = e.target.value; emit(); }, 350)
  );
}

/**
 * Get the current filter state.
 * @returns {object}
 */
export function getFilters() {
  return { ...state };
}

/**
 * Programmatically set filters and emit.
 * @param {Partial<typeof state>} filters
 */
export function setFilters(filters) {
  Object.assign(state, filters);
  emit();
}

// ── Internals ─────────────────────────────────────────────────────────────────

function setActiveChip(groupSelector, value) {
  document.querySelectorAll(`${groupSelector} .filter-chip`).forEach(chip => {
    chip.classList.toggle('active', chip.dataset.value === value);
  });
}

function emit() {
  document.dispatchEvent(new CustomEvent('filtersChanged', { detail: { ...state } }));
}

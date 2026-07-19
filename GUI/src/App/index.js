/**
 * App/index.js
 * Main application orchestrator.
 *
 * Responsibilities:
 * - Boot sequence (config → local country → initial data load)
 * - Periodic data refresh
 * - Wiring filters → data → map + process panel
 */

import { fetchConfig, fetchConnections, fetchLocalCountry, fetchProcesses } from '../api/client.js';
import { initializeMap, setLocalCountry, setConnections, updateConnections } from '../Map/index.js';
import { initProcessPanel, setProcessStatistics } from '../ProcessPanel/index.js';
import { initFilters, getFilters } from '../Filters/index.js';

// ── DOM refs ──────────────────────────────────────────────────────────────────
const mapContainer      = document.getElementById('map-container');
const filterContainer   = document.getElementById('filters-container');
const processPanelEl    = document.getElementById('process-panel-container');
const loadingOverlay    = document.getElementById('loading-overlay');
const loadingText       = document.getElementById('loading-text');
const localCountryLabel = document.getElementById('local-country-label');
const lastUpdatedEl     = document.getElementById('last-updated');
const rowCountEl        = document.getElementById('row-count');

// ── App state ─────────────────────────────────────────────────────────────────
let refreshInterval = 5000; // ms, overridden by config
let refreshTimer    = null;
let isFirstLoad     = true;
let totalRowCount   = 0;

// ── Boot ──────────────────────────────────────────────────────────────────────
async function boot() {
  setLoading('LOADING CONFIG…');

  // 1. Load config
  let config = { refreshInterval: 5000 };
  try {
    config = await fetchConfig();
    refreshInterval = config.refreshInterval || 5000;
  } catch (e) {
    console.warn('[App] Config fetch failed:', e.message);
  }

  // 2. Init UI components
  initFilters(filterContainer);
  initProcessPanel(processPanelEl);

  // 3. Init map
  setLoading('LOADING MAP…');
  try {
    await initializeMap(mapContainer);
  } catch (e) {
    console.error('[App] Map init failed:', e);
  }

  // 4. Detect local country
  setLoading('DETECTING ORIGIN…');
  let localCountry = 'Unknown';
  try {
    const lc = await fetchLocalCountry();
    localCountry = lc.country || 'Unknown';
  } catch (e) {
    console.warn('[App] Local country fetch failed:', e.message);
  }

  if (localCountry !== 'Unknown') {
    setLocalCountry(localCountry);
    if (localCountryLabel) localCountryLabel.textContent = localCountry;
  }

  // 5. Initial data load
  setLoading('LOADING TRAFFIC DATA…');
  await refreshData();

  // 6. Hide loading overlay
  hideLoading();
  isFirstLoad = false;

  // 7. Listen for filter changes
  document.addEventListener('filtersChanged', () => {
    clearTimeout(refreshTimer);
    refreshData().then(scheduleRefresh);
  });

  scheduleRefresh();
}

// ── Data refresh ──────────────────────────────────────────────────────────────
async function refreshData() {
  const filters = getFilters();
  const apiFilters = buildApiFilters(filters);

  const [connectionsResult, processesResult] = await Promise.allSettled([
    fetchConnections(apiFilters),
    fetchProcesses(apiFilters),
  ]);

  // Connections
  if (connectionsResult.status === 'fulfilled') {
    const data = connectionsResult.value;
    if (isFirstLoad) {
      setConnections(data);
    } else {
      updateConnections(data);
    }
    totalRowCount = data.reduce((s, d) => s + (d.packet_count || 0), 0);
    if (rowCountEl) {
      rowCountEl.textContent = `${totalRowCount.toLocaleString()} packets`;
    }
  } else {
    console.error('[App] Connections fetch error:', connectionsResult.reason);
  }

  // Processes
  if (processesResult.status === 'fulfilled') {
    setProcessStatistics(processesResult.value);
  } else {
    console.error('[App] Processes fetch error:', processesResult.reason);
  }

  // Update "last updated" timestamp
  if (lastUpdatedEl) {
    lastUpdatedEl.textContent = `Updated ${new Date().toLocaleTimeString()}`;
  }
}

function scheduleRefresh() {
  clearTimeout(refreshTimer);
  refreshTimer = setTimeout(async () => {
    await refreshData();
    scheduleRefresh();
  }, refreshInterval);
}

// ── Filter → API param mapping ────────────────────────────────────────────────
function buildApiFilters(filters) {
  const params = {};
  if (filters.direction !== 'both')      params.direction  = filters.direction;
  if (filters.protocol  !== 'all')       params.protocol   = filters.protocol;
  if (filters.timeRange !== 'all')       params.timeRange  = filters.timeRange;
  if (filters.process   && filters.process.trim())  params.process   = filters.process.trim();
  if (filters.country   && filters.country.trim())  params.country   = filters.country.trim();
  return params;
}

// ── Loading overlay ───────────────────────────────────────────────────────────
function setLoading(msg) {
  if (loadingText) loadingText.textContent = msg;
}
function hideLoading() {
  if (loadingOverlay) loadingOverlay.classList.add('hidden');
}

// ── Start ─────────────────────────────────────────────────────────────────────
boot().catch(err => {
  console.error('[App] Boot failed:', err);
  setLoading(`ERROR: ${err.message}`);
});

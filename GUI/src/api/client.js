/**
 * api/client.js
 * Thin fetch wrappers for the Vite SQLite API endpoints.
 * All filter state is passed as query string parameters.
 */

const BASE = '/api';

/**
 * Build a URL with query params.
 * @param {string} endpoint
 * @param {Record<string,any>} params
 * @returns {string}
 */
function buildUrl(endpoint, params = {}) {
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== null && v !== '') {
      qs.set(k, v);
    }
  }
  const q = qs.toString();
  return `${BASE}/${endpoint}${q ? '?' + q : ''}`;
}

/**
 * Fetch and parse JSON from an API endpoint.
 * @param {string} url
 * @returns {Promise<any>}
 */
async function apiFetch(url) {
  const res = await fetch(url);
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || `HTTP ${res.status}`);
  }
  return res.json();
}

/**
 * Fetch aggregated connection data (one entry per destination country).
 * @param {object} filters
 * @returns {Promise<ConnectionRow[]>}
 */
export async function fetchConnections(filters = {}) {
  return apiFetch(buildUrl('connections', filters));
}

/**
 * Fetch process bandwidth statistics.
 * @param {object} filters
 * @returns {Promise<{ processes: ProcessRow[], total_bytes: number }>}
 */
export async function fetchProcesses(filters = {}) {
  return apiFetch(buildUrl('processes', filters));
}

/**
 * Fetch the detected local country.
 * @returns {Promise<{ country: string }>}
 */
export async function fetchLocalCountry() {
  return apiFetch(buildUrl('local-country'));
}

/**
 * Fetch backend config.
 * @returns {Promise<{ dbPath: string, refreshInterval: number, theme: string }>}
 */
export async function fetchConfig() {
  return apiFetch(buildUrl('config'));
}

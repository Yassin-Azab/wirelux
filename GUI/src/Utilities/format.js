/**
 * Utilities/format.js
 * Human-readable formatting helpers.
 */

/**
 * Format bytes into a human-readable string.
 * @param {number} bytes
 * @returns {string}
 */
export function formatBytes(bytes) {
  if (!bytes || bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(Math.abs(bytes)) / Math.log(1024));
  const clamped = Math.min(i, units.length - 1);
  const val = bytes / Math.pow(1024, clamped);
  return `${val.toFixed(clamped === 0 ? 0 : 1)} ${units[clamped]}`;
}

/**
 * Format a number with thousands separators.
 * @param {number} n
 * @returns {string}
 */
export function formatNumber(n) {
  if (!n && n !== 0) return '—';
  return n.toLocaleString();
}

/**
 * Format a percentage with one decimal place.
 * @param {number} pct  0-100
 * @returns {string}
 */
export function formatPercent(pct) {
  return `${(+pct || 0).toFixed(1)}%`;
}

/**
 * Protocol number → label.
 * @param {number|string} proto
 * @returns {string}
 */
export function formatProtocol(proto) {
  const map = { 6: 'TCP', 17: 'UDP', 1: 'ICMP' };
  return map[+proto] || `Proto ${proto}`;
}

/**
 * Clamp a value between min and max.
 */
export function clamp(val, min, max) {
  return Math.max(min, Math.min(max, val));
}

/**
 * Debounce a function.
 */
export function debounce(fn, delay) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), delay);
  };
}

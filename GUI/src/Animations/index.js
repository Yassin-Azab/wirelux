/**
 * Animations/index.js
 * Shared animation helpers used by the Map and Process panel.
 */

/**
 * Fade an element in by setting opacity to 1.
 * @param {HTMLElement|SVGElement} el
 * @param {number} durationMs
 * @param {number} delayMs
 */
export function fadeIn(el, durationMs = 300, delayMs = 0) {
  el.style.transition = `opacity ${durationMs}ms ease ${delayMs}ms`;
  el.style.opacity = '0';
  requestAnimationFrame(() => {
    requestAnimationFrame(() => { el.style.opacity = '1'; });
  });
}

/**
 * Fade an element out then optionally remove it.
 * @param {HTMLElement|SVGElement} el
 * @param {number} durationMs
 * @param {boolean} remove
 */
export function fadeOut(el, durationMs = 250, remove = false) {
  el.style.transition = `opacity ${durationMs}ms ease`;
  el.style.opacity = '0';
  if (remove) {
    setTimeout(() => el.remove(), durationMs);
  }
}

/**
 * D3-compatible ease-in-out cubic (without importing d3-ease).
 */
export function easeInOutCubic(t) {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

/**
 * Animate a numeric value from `from` to `to` over `duration` ms.
 * @param {number} from
 * @param {number} to
 * @param {number} duration
 * @param {(value: number) => void} onUpdate
 * @param {() => void} [onDone]
 */
export function animateValue(from, to, duration, onUpdate, onDone) {
  const start = performance.now();
  function tick(now) {
    const t = Math.min((now - start) / duration, 1);
    const eased = easeInOutCubic(t);
    onUpdate(from + (to - from) * eased);
    if (t < 1) requestAnimationFrame(tick);
    else if (onDone) onDone();
  }
  requestAnimationFrame(tick);
}

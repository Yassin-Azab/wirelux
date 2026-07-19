/**
 * Map/Connections/index.js
 * Renders animated curved arcs between origin and destination countries.
 *
 * Each arc:
 *  - Is a cubic bezier curve (great-circle approximation in projected space)
 *  - Has an animated flowing dash offset
 *  - Glows via SVG filter
 *  - Fades in on creation, fades out on removal
 */

import * as d3 from 'd3';
import { showTooltip, moveTooltip, hideTooltip } from '../Tooltip/index.js';

// Arc appearance
const ARC_WIDTH       = 1.6;
const ARC_COLOR       = '#00d4ff';
const ANIM_SPEED      = 40; // pixels/second dash offset
const DASH_PATTERN    = '6 4'; // dash, gap

let arcsGroup  = null;
let originPos  = null; // [x, y] in projected space

/**
 * Initialise the arcs layer.
 * @param {d3.Selection} parentGroup
 */
export function initConnections(parentGroup) {
  arcsGroup = parentGroup.append('g').attr('class', 'arcs-layer');
}

/**
 * Set the origin centroid position.
 * @param {[number, number]} pos  Projected [x, y]
 */
export function setOriginPosition(pos) {
  originPos = pos;
}

/**
 * Render or update arcs for the given connection list.
 * @param {object[]} connections  Aggregated connection rows (one per country)
 * @param {Map<string, [number,number]>} centroidMap  country→projected centroid
 */
export function renderConnections(connections, centroidMap) {
  if (!arcsGroup || !originPos) return;

  // Filter connections that have a valid centroid
  const valid = connections.filter(d => centroidMap.has(d.country));

  const sel = arcsGroup
    .selectAll('.arc-group')
    .data(valid, d => d.country);

  // ── Enter ──────────────────────────────────────────────────────────────────
  const entered = sel.enter()
    .append('g')
    .attr('class', 'arc-group')
    .style('opacity', 0);

  // Shadow / glow arc
  entered.append('path')
    .attr('class', 'arc-glow')
    .attr('fill', 'none')
    .attr('stroke', ARC_COLOR)
    .attr('stroke-width', ARC_WIDTH + 3)
    .attr('stroke-linecap', 'round')
    .attr('stroke-opacity', 0.18)
    .attr('filter', 'url(#glow-cyan)')
    .attr('d', d => buildArcPath(originPos, centroidMap.get(d.country)));

  // Main animated arc
  entered.append('path')
    .attr('class', 'arc-main')
    .attr('fill', 'none')
    .attr('stroke', ARC_COLOR)
    .attr('stroke-width', ARC_WIDTH)
    .attr('stroke-linecap', 'round')
    .attr('stroke-dasharray', DASH_PATTERN)
    .attr('d', d => buildArcPath(originPos, centroidMap.get(d.country)))
    .attr('stroke-dashoffset', 0);

  // Interaction overlay (thick invisible hit area)
  entered.append('path')
    .attr('class', 'arc-hit')
    .attr('fill', 'none')
    .attr('stroke', 'transparent')
    .attr('stroke-width', 14)
    .attr('d', d => buildArcPath(originPos, centroidMap.get(d.country)))
    .style('cursor', 'crosshair')
    .on('mouseenter', function(event, d) {
      d3.select(this.parentNode).select('.arc-main')
        .transition().duration(120)
        .attr('stroke', '#ffffff')
        .attr('stroke-width', ARC_WIDTH + 1.5)
        .attr('stroke-opacity', 1);
      showTooltip({ x: event.clientX, y: event.clientY }, d);
    })
    .on('mousemove', function(event) {
      moveTooltip(event.clientX, event.clientY);
    })
    .on('mouseleave', function() {
      d3.select(this.parentNode).select('.arc-main')
        .transition().duration(200)
        .attr('stroke', ARC_COLOR)
        .attr('stroke-width', ARC_WIDTH)
        .attr('stroke-opacity', 1);
      hideTooltip(80);
    });

  // Fade in
  entered.transition().duration(600).ease(d3.easeCubicOut).style('opacity', 1);

  // ── Exit ───────────────────────────────────────────────────────────────────
  sel.exit()
    .transition().duration(400)
    .style('opacity', 0)
    .remove();

  // ── Update paths (in case centroid changed after resize) ───────────────────
  const merged = sel.merge(entered);
  merged.select('.arc-glow').attr('d', d => buildArcPath(originPos, centroidMap.get(d.country)));
  merged.select('.arc-main').attr('d', d => buildArcPath(originPos, centroidMap.get(d.country)));
  merged.select('.arc-hit').attr('d', d => buildArcPath(originPos, centroidMap.get(d.country)));

  // ── Animate all active arcs ────────────────────────────────────────────────
  startArcAnimation();
}

/**
 * Clear all arcs.
 */
export function clearConnections() {
  if (arcsGroup) {
    arcsGroup.selectAll('.arc-group')
      .transition().duration(350)
      .style('opacity', 0)
      .remove();
  }
}

// ── Arc path builder ──────────────────────────────────────────────────────────
/**
 * Build a quadratic bezier arc path between two projected points.
 * The control point is lifted perpendicular to the midpoint.
 */
function buildArcPath(from, to) {
  if (!from || !to) return '';
  const [x1, y1] = from;
  const [x2, y2] = to;
  const mx = (x1 + x2) / 2;
  const my = (y1 + y2) / 2;
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  // Perpendicular offset scales with distance — tighter for close countries
  const offset = Math.min(dist * 0.28, 120);
  // Perpendicular direction (rotate 90°)
  const nx = -dy / dist * offset;
  const ny =  dx / dist * offset;
  // Control point bows "upward" in screen space
  const cx = mx + nx;
  const cy = my + ny - offset * 0.4;
  return `M${x1},${y1} Q${cx},${cy} ${x2},${y2}`;
}

// ── Continuous dash-offset animation ─────────────────────────────────────────
let _animRunning = false;
let _lastTime    = 0;
let _offset      = 0;

function startArcAnimation() {
  if (_animRunning) return;
  _animRunning = true;
  _lastTime = performance.now();
  requestAnimationFrame(tick);
}

function tick(now) {
  const dt = (now - _lastTime) / 1000;
  _lastTime = now;
  _offset -= ANIM_SPEED * dt;

  if (arcsGroup) {
    arcsGroup.selectAll('.arc-main')
      .attr('stroke-dashoffset', _offset);
  }

  const hasArcs = arcsGroup && !arcsGroup.selectAll('.arc-group').empty();
  if (hasArcs) {
    requestAnimationFrame(tick);
  } else {
    _animRunning = false;
  }
}

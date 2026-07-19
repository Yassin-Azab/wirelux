/**
 * Map/Nodes/index.js
 * Renders glowing country nodes on the map at centroid positions.
 * Only shows nodes for countries with active connections.
 */

import * as d3 from 'd3';

const NODE_R       = 5;
const ORIGIN_R     = 7;
const HOVER_SCALE  = 1.6;
const PULSE_PERIOD = 2200; // ms

let nodesGroup  = null;
let originGroup = null;
let geoPath     = null;
let featureMap  = null;

/**
 * Initialise the nodes layer inside the given SVG <g>.
 * @param {d3.Selection} parentGroup  The SVG <g> for the map layer.
 * @param {d3.GeoPath}   path         Configured geo path generator.
 * @param {Map}          fMap         id→feature map.
 */
export function initNodes(parentGroup, path, fMap) {
  geoPath    = path;
  featureMap = fMap;

  nodesGroup  = parentGroup.append('g').attr('class', 'nodes-layer');
  originGroup = parentGroup.append('g').attr('class', 'origin-layer');
}

/**
 * Update the geo path generator after a resize.
 * @param {d3.GeoPath} path
 */
export function updateGeoPath(path) {
  geoPath = path;
}

/**
 * Render (or update) the origin country node.
 * @param {object} feature  GeoJSON feature for the origin country.
 */
export function renderOriginNode(feature) {
  if (!feature) return;

  const centroid = geoPath.centroid(feature);
  if (!centroid || isNaN(centroid[0])) return;

  originGroup.selectAll('.origin-node').remove();

  const g = originGroup.append('g')
    .attr('class', 'origin-node')
    .attr('transform', `translate(${centroid[0]}, ${centroid[1]})`)
    .style('opacity', 0);

  // Outer pulse ring
  g.append('circle')
    .attr('class', 'origin-pulse')
    .attr('r', ORIGIN_R + 4)
    .attr('fill', 'none')
    .attr('stroke', 'var(--color-node-origin)')
    .attr('stroke-width', 1)
    .attr('opacity', 0.6);

  // Core dot
  g.append('circle')
    .attr('r', ORIGIN_R)
    .attr('fill', 'var(--color-node-origin)')
    .attr('filter', 'url(#glow-green)');

  // Fade in
  g.transition().duration(600).style('opacity', 1);

  // Pulsing animation
  animatePulse(g.select('.origin-pulse'), ORIGIN_R + 4, ORIGIN_R + 14);
}

/**
 * Render destination country nodes.
 * @param {Array<{ country: string, data: object }>} connections
 * @param {Map<string, [number,number]>} centroidMap  country→projected centroid
 */
export function renderNodes(connections, centroidMap) {
  if (!nodesGroup) return;

  const keys = new Set(connections.map(c => c.country));

  // Data join
  const sel = nodesGroup
    .selectAll('.dest-node')
    .data(connections, d => d.country);

  // ── Enter ──────────────────────────────────────────────────────────────────
  const enter = sel.enter()
    .append('g')
    .attr('class', 'dest-node')
    .attr('transform', d => {
      const c = centroidMap.get(d.country);
      return c ? `translate(${c[0]}, ${c[1]})` : 'translate(-9999,-9999)';
    })
    .style('opacity', 0)
    .style('cursor', 'default');

  enter.append('circle')
    .attr('class', 'node-pulse')
    .attr('r', NODE_R + 3)
    .attr('fill', 'none')
    .attr('stroke', 'var(--color-accent)')
    .attr('stroke-width', 0.8)
    .attr('opacity', 0.5);

  enter.append('circle')
    .attr('class', 'node-core')
    .attr('r', NODE_R)
    .attr('fill', 'var(--color-accent)')
    .attr('filter', 'url(#glow-cyan)');

  enter.transition().duration(500)
    .style('opacity', 1)
    .select('.node-core')
    .attr('r', NODE_R);

  // Pulse each new node
  enter.each(function() {
    const g = d3.select(this);
    animatePulse(g.select('.node-pulse'), NODE_R + 3, NODE_R + 12);
  });

  // Hover effect
  enter
    .on('mouseenter', function() {
      d3.select(this).select('.node-core')
        .transition().duration(150)
        .attr('r', NODE_R * HOVER_SCALE)
        .attr('fill', '#ffffff');
    })
    .on('mouseleave', function() {
      d3.select(this).select('.node-core')
        .transition().duration(200)
        .attr('r', NODE_R)
        .attr('fill', 'var(--color-accent)');
    });

  // ── Exit ───────────────────────────────────────────────────────────────────
  sel.exit()
    .transition().duration(400)
    .style('opacity', 0)
    .remove();

  // ── Update ─────────────────────────────────────────────────────────────────
  sel.merge(enter)
    .attr('transform', d => {
      const c = centroidMap.get(d.country);
      return c ? `translate(${c[0]}, ${c[1]})` : 'translate(-9999,-9999)';
    });
}

/**
 * Animate a pulsing ring forever.
 */
function animatePulse(sel, rMin, rMax) {
  function loop() {
    sel
      .attr('r', rMin)
      .attr('opacity', 0.7)
      .transition()
      .duration(PULSE_PERIOD)
      .ease(d3.easeSinOut)
      .attr('r', rMax)
      .attr('opacity', 0)
      .on('end', loop);
  }
  loop();
}

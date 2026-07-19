/**
 * Map/index.js
 * Main map orchestrator — public API surface.
 *
 * Public API:
 *   initializeMap(container)
 *   setLocalCountry(country)
 *   setConnections(connectionData)
 *   updateConnections(connectionData)
 *   clearConnections()
 *   zoomToCountry(country)
 *   highlightCountry(country)
 *   resetView()
 */

import * as d3 from 'd3';
import * as topojson from 'topojson-client';
import { createProjection, createZoom } from './Projection/index.js';
import { countryNameToId, countryIdToName, buildFeatureMap } from './CountryLookup/index.js';
import { initNodes, renderOriginNode, renderNodes, updateGeoPath } from './Nodes/index.js';
import {
  initConnections,
  setOriginPosition,
  renderConnections,
  clearConnections as clearArcs,
} from './Connections/index.js';

// ── Module state ──────────────────────────────────────────────────────────────
let svg          = null;
let mapGroup     = null;
let geoPath      = null;
let projection   = null;
let zoomBehavior = null;
let resetViewFn  = null;
let featureMap   = new Map();      // numeric id → GeoJSON feature
let centroidMap  = new Map();      // country name → [x, y]
let worldData    = null;
let originFeature = null;
let _localCountry = null;
let _connections  = [];

// ── Glow / defs ───────────────────────────────────────────────────────────────
function injectDefs(svgSel) {
  const defs = svgSel.append('defs');

  function glowFilter(id, stdDev) {
    const f = defs.append('filter')
      .attr('id', id)
      .attr('x', '-60%').attr('y', '-60%')
      .attr('width', '220%').attr('height', '220%');
    f.append('feGaussianBlur').attr('stdDeviation', stdDev).attr('result', 'blur');
    const m = f.append('feMerge');
    m.append('feMergeNode').attr('in', 'blur');
    m.append('feMergeNode').attr('in', 'SourceGraphic');
    return f;
  }

  glowFilter('glow-cyan', 3.5);
  glowFilter('glow-green', 5);

  // Radial ocean gradient
  const og = defs.append('radialGradient')
    .attr('id', 'ocean-grad')
    .attr('cx', '50%').attr('cy', '55%').attr('r', '70%');
  og.append('stop').attr('offset', '0%').attr('stop-color', '#071828');
  og.append('stop').attr('offset', '100%').attr('stop-color', '#030810');
}

// ── World-atlas loader ────────────────────────────────────────────────────────
async function loadWorldAtlas() {
  // Served by the worldAtlasPlugin in vite.config.js
  const urls = [
    '/world-atlas/countries-110m.json',
    '/node_modules/world-atlas/countries-110m.json',
  ];
  for (const url of urls) {
    try {
      const res = await fetch(url);
      if (res.ok) {
        console.log(`[Map] Loaded world-atlas from ${url}`);
        return await res.json();
      }
    } catch { /* try next */ }
  }
  throw new Error('Could not load world-atlas topology from any path');
}

// ── Public API ────────────────────────────────────────────────────────────────

/**
 * Initialize and render the world map inside `container`.
 * @param {HTMLElement} container
 */
export async function initializeMap(container) {
  const w = container.clientWidth  || 800;
  const h = container.clientHeight || 500;

  // ── SVG ─────────────────────────────────────────────────────────────────
  svg = d3.select(container).append('svg')
    .attr('width', w)
    .attr('height', h)
    .style('display', 'block')
    .style('background', '#04101e');

  injectDefs(svg);

  // Full-size ocean background
  svg.append('rect')
    .attr('class', 'ocean-bg')
    .attr('width', w)
    .attr('height', h)
    .attr('fill', 'url(#ocean-grad)');

  // Root group for zoom transforms
  mapGroup = svg.append('g').attr('class', 'map-root');

  // ── Load topology ────────────────────────────────────────────────────────
  worldData = await loadWorldAtlas();
  const countries = topojson.feature(worldData, worldData.objects.countries);
  featureMap = buildFeatureMap(countries);

  // ── Projection & path ────────────────────────────────────────────────────
  projection = createProjection(w, h);
  geoPath    = d3.geoPath().projection(projection);

  // ── Build centroid lookup keyed by country name ──────────────────────────
  buildCentroidMap(countries.features);

  // ── Draw base map layers ─────────────────────────────────────────────────
  // Sphere fill (ocean colour inside sphere boundary)
  mapGroup.append('path')
    .datum({ type: 'Sphere' })
    .attr('class', 'sphere-fill')
    .attr('fill', '#04101e')
    .attr('stroke', 'rgba(0,212,255,0.15)')
    .attr('stroke-width', 0.8)
    .attr('d', geoPath);

  // Graticule grid
  mapGroup.append('path')
    .datum(d3.geoGraticule()())
    .attr('class', 'graticule')
    .attr('fill', 'none')
    .attr('stroke', 'rgba(0,212,255,0.05)')
    .attr('stroke-width', 0.4)
    .attr('d', geoPath);

  // Country fills
  mapGroup.append('g')
    .attr('class', 'countries-layer')
    .selectAll('path.country')
    .data(countries.features)
    .join('path')
    .attr('class', 'country')
    .attr('d', geoPath)
    .attr('fill', '#0d1e33')
    .attr('stroke', 'rgba(0,212,255,0.12)')
    .attr('stroke-width', 0.4)
    .on('mouseenter', function() {
      const d = d3.select(this).datum();
      if (originFeature && +d.id === +originFeature.id) return;
      d3.select(this).attr('fill', '#16273f');
    })
    .on('mouseleave', function() {
      const d = d3.select(this).datum();
      if (originFeature && +d.id === +originFeature.id) return;
      d3.select(this).attr('fill', '#0d1e33');
    });

  // ── Sub-layers (arcs above countries, nodes on top) ──────────────────────
  initConnections(mapGroup);
  initNodes(mapGroup, geoPath, featureMap);

  // ── Zoom / pan ───────────────────────────────────────────────────────────
  const zr = createZoom(svg, mapGroup);
  zoomBehavior = zr.zoom;
  resetViewFn  = zr.resetView;

  // ── Responsive resize ────────────────────────────────────────────────────
  const ro = new ResizeObserver(() => onResize(container));
  ro.observe(container);

  console.log('[Map] Initialized. Features:', countries.features.length);
}

/**
 * Set the local (origin) country.
 * @param {string} country
 */
export function setLocalCountry(country) {
  _localCountry = country;
  const id = countryNameToId(country);
  if (id == null) { console.warn('[Map] Unknown country:', country); return; }

  originFeature = featureMap.get(id);
  if (!originFeature) { console.warn('[Map] Feature not found for id', id, country); return; }

  // Highlight on the map
  mapGroup.selectAll('path.country')
    .filter(d => +d.id === +originFeature.id)
    .attr('fill', 'rgba(0,255,136,0.16)')
    .attr('stroke', 'rgba(0,255,136,0.5)')
    .attr('stroke-width', 0.8);

  const c = centroidMap.get(country);
  if (c) setOriginPosition(c);

  renderOriginNode(originFeature);
}

/**
 * Set connections (first load).
 * @param {object[]} connectionData
 */
export function setConnections(connectionData) {
  _connections = connectionData || [];
  _renderOverlays();
}

/**
 * Update connections (smooth diff).
 * @param {object[]} connectionData
 */
export function updateConnections(connectionData) {
  _connections = connectionData || [];
  _renderOverlays();
}

/**
 * Clear all arcs and destination nodes.
 */
export function clearConnections() {
  _connections = [];
  clearArcs();
}

/**
 * Zoom to a named country.
 * @param {string} country
 */
export function zoomToCountry(country) {
  const id = countryNameToId(country);
  if (id == null || !svg) return;
  const feature = featureMap.get(id);
  if (!feature) return;

  const [[x0, y0], [x1, y1]] = geoPath.bounds(feature);
  const w  = +svg.attr('width');
  const h  = +svg.attr('height');
  const scale = Math.min(8, 0.85 / Math.max((x1 - x0) / w, (y1 - y0) / h));
  const cx = (x0 + x1) / 2;
  const cy = (y0 + y1) / 2;

  svg.transition().duration(750).ease(d3.easeCubicInOut)
    .call(zoomBehavior.transform,
      d3.zoomIdentity.translate(w / 2, h / 2).scale(scale).translate(-cx, -cy)
    );
}

/**
 * Flash-highlight a country.
 * @param {string} country
 */
export function highlightCountry(country) {
  const id = countryNameToId(country);
  if (id == null) return;
  mapGroup.selectAll('path.country')
    .filter(d => +d.id === id)
    .transition().duration(200).attr('fill', 'rgba(0,212,255,0.4)')
    .transition().duration(600).attr('fill', '#0d1e33');
}

/**
 * Reset map zoom to default.
 */
export function resetView() {
  if (resetViewFn) resetViewFn();
}

// ── Internals ─────────────────────────────────────────────────────────────────

function buildCentroidMap(features) {
  centroidMap = new Map();
  for (const feature of features) {
    const name = countryIdToName(feature.id);
    if (!name) continue;
    const c = geoPath.centroid(feature);
    if (c && !isNaN(c[0]) && !isNaN(c[1])) {
      centroidMap.set(name, c);
    }
  }
}

function _renderOverlays() {
  renderConnections(_connections, centroidMap);
  renderNodes(_connections, centroidMap);
}

function onResize(container) {
  if (!svg || !worldData) return;

  const w = container.clientWidth;
  const h = container.clientHeight;
  if (w < 10 || h < 10) return;

  svg.attr('width', w).attr('height', h);
  svg.select('.ocean-bg').attr('width', w).attr('height', h);

  // Rebuild projection to fit new dimensions
  projection = createProjection(w, h);
  geoPath    = d3.geoPath().projection(projection);
  updateGeoPath(geoPath);

  // Redraw base layers
  mapGroup.selectAll('.sphere-fill').attr('d', geoPath({ type: 'Sphere' }));
  mapGroup.selectAll('.graticule').attr('d', geoPath(d3.geoGraticule()()));
  mapGroup.selectAll('path.country').attr('d', geoPath);

  // Rebuild centroids
  const countries = topojson.feature(worldData, worldData.objects.countries);
  buildCentroidMap(countries.features);

  // Update arc origin position
  if (_localCountry) {
    const c = centroidMap.get(_localCountry);
    if (c) setOriginPosition(c);
  }

  // Re-render overlays
  _renderOverlays();

  // Re-render origin node with fresh geoPath
  if (originFeature) renderOriginNode(originFeature);
}

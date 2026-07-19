/**
 * Map/Projection/index.js
 * Sets up and manages the D3 map projection and zoom behavior.
 */

import * as d3 from 'd3';

/**
 * Create a Natural Earth projection that properly fits the given dimensions.
 * Uses D3's fitExtent so the math is always correct.
 * @param {number} width
 * @param {number} height
 * @returns {d3.GeoProjection}
 */
export function createProjection(width, height) {
  const padding = 20;
  return d3.geoNaturalEarth1()
    .fitExtent(
      [[padding, padding], [width - padding, height - padding]],
      { type: 'Sphere' }
    );
}

/**
 * Create a D3 zoom behavior.
 * - Drag to pan (mouse / touch)
 * - Double-click to reset
 * - NO scroll-to-zoom
 *
 * @param {d3.Selection} svg       Root SVG element selection.
 * @param {d3.Selection} mapGroup  The <g> that receives transform.
 * @returns {{ zoom: d3.ZoomBehavior, resetView: () => void }}
 */
export function createZoom(svg, mapGroup) {
  const zoom = d3.zoom()
    .scaleExtent([1, 20])
    // Only allow drag (pointer events), block wheel / scroll zoom
    .filter(event => {
      if (event.type === 'wheel') return false;
      if (event.type === 'dblclick') return false; // we handle dblclick manually
      return true;
    })
    .on('zoom', event => {
      mapGroup.attr('transform', event.transform);
    });

  svg.call(zoom);

  // Disable the default dblclick zoom that D3 attaches separately
  svg.on('dblclick.zoom', null);

  // Double-click → reset to identity
  svg.on('dblclick', () => resetView());

  function resetView() {
    svg.transition()
      .duration(650)
      .ease(d3.easeCubicInOut)
      .call(zoom.transform, d3.zoomIdentity);
  }

  return { zoom, resetView };
}

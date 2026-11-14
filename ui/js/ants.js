// Marching ants, shared by the ATLAS selection frame and the RIP shape outlines (design D5
// of add-quad-shapes; look from add-atlas-marquee): a 1 CSS px white line with black 4 px
// dashes over it that shift 1 px every 50 ms. Named deviation: black and white are not
// O'Tool tokens.

export const ANTS = { dash: 4, stepMs: 50, dark: '#000000', light: '#FFFFFF' };

/**
 * Strokes the context's current path as marching ants. `unit` is one CSS pixel in the
 * context's coordinates: devicePixelRatio for an untransformed canvas, 1 when the context
 * is already scaled by it.
 */
export function strokeAnts(g, unit) {
    const period = 2 * ANTS.dash;
    g.lineWidth = unit;
    g.setLineDash([]);
    g.strokeStyle = ANTS.light;
    g.stroke();
    g.setLineDash([ANTS.dash * unit, ANTS.dash * unit]);
    g.lineDashOffset = -(Math.floor(performance.now() / ANTS.stepMs) % period) * unit;
    g.strokeStyle = ANTS.dark;
    g.stroke();
    g.setLineDash([]);
    g.lineDashOffset = 0;
}

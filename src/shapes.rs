//! Round shapes for the HUD, built from quads (the only non-rectangle the GUI draws).
//!
//! A disc is a fan of thin wedges around its center; a ring is a band of quads between
//! two circles. Neighbouring pieces share edges without overlapping, so a see-through
//! colour stays even across the shape.

use std::f32::consts::TAU;

/// A quad's four corners in GUI pixels, in order round it.
pub type Quad = [[f32; 2]; 4];

/// How many pieces make a circle: enough to look round at HUD sizes.
const SEGMENTS: usize = 32;

/// A filled circle of `radius` around `center`.
pub fn disc(center: [f32; 2], radius: f32) -> Vec<Quad> {
    if !(radius > 0.0) {
        return Vec::new();
    }
    (0..SEGMENTS)
        .map(|i| {
            let start = point(center, radius, i as f32);
            let middle = point(center, radius, i as f32 + 0.5);
            let end = point(center, radius, i as f32 + 1.0);
            [center, start, middle, end]
        })
        .collect()
}

/// A ring from radius `inner` out to `outer` around `center`.
pub fn ring(center: [f32; 2], inner: f32, outer: f32) -> Vec<Quad> {
    if !(outer > inner && inner >= 0.0) {
        return Vec::new();
    }
    (0..SEGMENTS)
        .map(|i| {
            let (from, to) = (i as f32, i as f32 + 1.0);
            [point(center, outer, from), point(center, outer, to), point(center, inner, to), point(center, inner, from)]
        })
        .collect()
}

/// The point at `step` (of `SEGMENTS` around) on the circle of `radius` around `center`.
fn point(center: [f32; 2], radius: f32, step: f32) -> [f32; 2] {
    let angle = step / SEGMENTS as f32 * TAU;
    [center[0] + radius * angle.cos(), center[1] + radius * angle.sin()]
}

/// `0xRRGGBB` with `opacity` (0..1) as `0xAARRGGBB`.
pub fn with_opacity(rgb: u32, opacity: f32) -> u32 {
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    (alpha << 24) | (rgb & 0x00FF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
        (a[0] - b[0]).hypot(a[1] - b[1])
    }

    #[test]
    fn disc_points_lie_on_the_circle() {
        let center = [10.0, 20.0];
        let quads = disc(center, 3.0);
        assert_eq!(quads.len(), SEGMENTS);
        for quad in &quads {
            assert_eq!(quad[0], center);
            for corner in &quad[1..] {
                assert!((distance(*corner, center) - 3.0).abs() < 1e-4);
            }
        }
        // The fan closes: the last wedge ends where the first starts.
        let (first, last) = (quads[0][1], quads[SEGMENTS - 1][3]);
        assert!(distance(first, last) < 1e-4);
    }

    #[test]
    fn ring_spans_inner_to_outer() {
        let quads = ring([0.0, 0.0], 3.0, 4.0);
        assert_eq!(quads.len(), SEGMENTS);
        for quad in &quads {
            assert!((distance(quad[0], [0.0, 0.0]) - 4.0).abs() < 1e-4);
            assert!((distance(quad[3], [0.0, 0.0]) - 3.0).abs() < 1e-4);
        }
    }

    #[test]
    fn empty_shapes_draw_nothing() {
        assert!(disc([0.0, 0.0], 0.0).is_empty());
        assert!(ring([0.0, 0.0], 3.0, 3.0).is_empty());
    }

    #[test]
    fn opacity_becomes_alpha() {
        assert_eq!(with_opacity(0xFFFFFF, 1.0), 0xFFFF_FFFF);
        assert_eq!(with_opacity(0x123456, 0.0), 0x0012_3456);
        assert_eq!(with_opacity(0xFF3030, 0.5) >> 24, 128);
    }
}

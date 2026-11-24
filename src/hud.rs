//! What the player sees of the lock: a dot on the target's chest, and no crosshair.

use mod_api::{ClientHost, DrawContext};

/// The HUD layer of the normal crosshair, hidden while locked (the dot replaces it).
pub const CROSSHAIR_LAYER: &str = "minecraft:crosshair";

/// The lock dot: size in GUI pixels and colour (0xAARRGGBB).
const DOT_SIZE: f32 = 4.0;
const DOT_COLOR: u32 = 0xF0FF_FFFF;
const DOT_BORDER_COLOR: u32 = 0xC000_0000;

/// Draw the lock dot at the world position `point`, if it is on screen.
pub fn draw_lock_dot(host: &dyn ClientHost, draw: &mut dyn DrawContext, point: [f64; 3]) {
    let Some([x, y, _]) = host.project(point) else {
        return;
    };
    let [width, height] = draw.size();
    let center = [(x + 1.0) / 2.0 * width, (1.0 - y) / 2.0 * height];

    let border = DOT_SIZE + 2.0;
    draw.fill([center[0] - border / 2.0, center[1] - border / 2.0, border, border], DOT_BORDER_COLOR);
    draw.fill([center[0] - DOT_SIZE / 2.0, center[1] - DOT_SIZE / 2.0, DOT_SIZE, DOT_SIZE], DOT_COLOR);
}

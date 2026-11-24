//! What the player sees of the lock: a round dot on the target's chest, and no crosshair.
//!
//! The dot's size, opacity, colour and outline are settings (`[hud]` in `config.rs`).

use mod_api::{ClientHost, DrawContext};

use crate::config;
use crate::shapes;

/// The HUD layer of the normal crosshair, hidden while locked (the dot replaces it).
pub const CROSSHAIR_LAYER: &str = "minecraft:crosshair";
/// Colour of the outline ring around the dot (it takes the dot's opacity).
const OUTLINE_RGB: u32 = 0x000000;

/// Draw the lock dot at the world position `point`, if it is on screen.
pub fn draw_lock_dot(host: &dyn ClientHost, draw: &mut dyn DrawContext, point: [f64; 3]) {
    let Some([x, y, _]) = host.project(point) else {
        return;
    };
    let [width, height] = draw.size();
    let center = [(x + 1.0) / 2.0 * width, (1.0 - y) / 2.0 * height];

    let config = config::get();
    let fill = shapes::with_opacity(config.dot_color, config.dot_opacity);
    for quad in shapes::disc(center, config.dot_size) {
        draw.quad(quad, fill);
    }
    let outline = shapes::with_opacity(OUTLINE_RGB, config.dot_opacity);
    for quad in shapes::ring(center, config.dot_size, config.dot_size + config.dot_outline) {
        draw.quad(quad, outline);
    }
}

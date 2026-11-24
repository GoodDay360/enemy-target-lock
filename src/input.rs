//! Raw input codes and helpers used by the hooks.

/// GLFW button number of the middle mouse button (lock / release).
pub const MIDDLE_MOUSE_BUTTON: i32 = 2;

/// Which way to switch targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Scroll wheel direction as a target switch, the same way the hotbar moves:
/// wheel down = right, wheel up = left.
pub fn scroll_side(scroll_y: f64) -> Option<Side> {
    if scroll_y < 0.0 {
        Some(Side::Right)
    } else if scroll_y > 0.0 {
        Some(Side::Left)
    } else {
        None
    }
}

/// Mouse sensitivity that stops the mouse from turning the camera.
///
/// The game turns the view by `(sensitivity * 0.6 + 0.2)^3 * 8` per mouse pixel, so -1/3
/// makes that zero. Used only for the frame's turn; the saved option is unchanged.
pub const MUTED_MOUSE_SENSITIVITY: f64 = -1.0 / 3.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_maps_to_sides() {
        assert_eq!(scroll_side(-1.0), Some(Side::Right));
        assert_eq!(scroll_side(0.5), Some(Side::Left));
        assert_eq!(scroll_side(0.0), None);
    }

    #[test]
    fn muted_sensitivity_gives_zero_turn() {
        let factor = MUTED_MOUSE_SENSITIVITY * 0.6 + 0.2;
        assert!((factor.powi(3) * 8.0).abs() < 1e-12);
    }
}

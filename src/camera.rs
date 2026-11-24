//! Aim math: where to look, and how far to turn each tick.

/// Subtle assist (no lock): only acts within this many degrees of the target...
pub const ASSIST_MAX_ANGLE_DEGREES: f32 = 8.0;
/// ...and turns at most this much per tick.
pub const ASSIST_STEP_DEGREES: f32 = 0.25;
/// How far up the target's bounding box to aim (0 = feet, 1 = top): roughly the chest.
const AIM_HEIGHT: f64 = 0.65;

/// Where to look to aim at a target, and how far away it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aim {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f64,
}

/// The point on a target's bounding box the camera aims at (chest height, centered).
pub fn aim_point(bounds: [[f64; 3]; 2]) -> [f64; 3] {
    [
        (bounds[0][0] + bounds[1][0]) / 2.0,
        bounds[0][1] + (bounds[1][1] - bounds[0][1]) * AIM_HEIGHT,
        (bounds[0][2] + bounds[1][2]) / 2.0,
    ]
}

/// The aim from `eye` at `point`.
/// Returns `None` when the point is directly above/below or the values are invalid.
pub fn aim_at(eye: [f64; 3], point: [f64; 3]) -> Option<Aim> {
    let dx = point[0] - eye[0];
    let dy = point[1] - eye[1];
    let dz = point[2] - eye[2];
    let horizontal = dx.hypot(dz);
    if !horizontal.is_finite() || horizontal == 0.0 {
        return None;
    }

    Some(Aim {
        yaw: (-dx).atan2(dz).to_degrees() as f32,
        pitch: -dy.atan2(horizontal).to_degrees() as f32,
        distance: horizontal.hypot(dy),
    })
}

/// Shortest signed yaw difference from `from` to `to`, in -180..180 degrees
/// (positive = to the right).
pub fn yaw_delta(from: f32, to: f32) -> f32 {
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

/// Total angle between a view direction and a target direction, in degrees.
pub fn angle_between(yaw: f32, pitch: f32, target_yaw: f32, target_pitch: f32) -> f32 {
    yaw_delta(yaw, target_yaw).hypot(target_pitch - pitch)
}

/// Turn from `(yaw, pitch)` toward `(target_yaw, target_pitch)` by at most `max_step` degrees,
/// taking the short way around. Returns `None` when already on target.
pub fn step_toward(yaw: f32, pitch: f32, target_yaw: f32, target_pitch: f32, max_step: f32) -> Option<[f32; 2]> {
    let yaw_d = yaw_delta(yaw, target_yaw);
    let pitch_d = target_pitch - pitch;
    let angle = yaw_d.hypot(pitch_d);
    if !angle.is_finite() || angle < 0.01 {
        return None;
    }
    let scale = angle.min(max_step) / angle;
    Some([yaw + yaw_d * scale, (pitch + pitch_d * scale).clamp(-90.0, 90.0)])
}

/// The subtle assist: a small `step_toward`, only when the target is already near the crosshair.
pub fn assist_step(yaw: f32, pitch: f32, target_yaw: f32, target_pitch: f32) -> Option<[f32; 2]> {
    if angle_between(yaw, pitch, target_yaw, target_pitch) > ASSIST_MAX_ANGLE_DEGREES {
        return None;
    }
    step_toward(yaw, pitch, target_yaw, target_pitch, ASSIST_STEP_DEGREES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_delta_uses_shortest_path_across_wraparound() {
        assert!((yaw_delta(-179.0, 179.0) - -2.0).abs() < 0.001);
        let step = assist_step(-179.0, 0.0, 179.0, 0.0).unwrap();
        assert!((step[0] - -179.25).abs() < 0.001);
    }

    #[test]
    fn assist_is_limited_and_stops_outside_the_assist_cone() {
        let step = assist_step(0.0, 0.0, 5.0, 0.0).unwrap();
        assert!((step[0] - ASSIST_STEP_DEGREES).abs() < 0.001);
        assert!(assist_step(0.0, 0.0, 9.0, 0.0).is_none());
    }

    #[test]
    fn step_toward_is_limited_without_a_cone() {
        let step = step_toward(0.0, 0.0, 90.0, 0.0, 6.0).unwrap();
        assert!((step[0] - 6.0).abs() < 0.001);
        let close = step_toward(0.0, 0.0, 2.0, 0.0, 6.0).unwrap();
        assert!((close[0] - 2.0).abs() < 0.001);
        assert!(step_toward(1.0, 1.0, 1.0, 1.0, 6.0).is_none());
    }

    #[test]
    fn pitch_is_clamped() {
        let step = step_toward(0.0, 89.0, 0.0, 120.0, 60.0).unwrap();
        assert_eq!(step[1], 90.0);
    }

    #[test]
    fn aims_at_chest_height_straight_ahead() {
        let point = aim_point([[-0.5, 0.0, 4.5], [0.5, 2.0, 5.5]]);
        assert_eq!(point, [0.0, 1.3, 5.0]);
        let aim = aim_at([0.0, 1.3, 0.0], point).unwrap();
        assert!(aim.yaw.abs() < 0.001);
        assert!(aim.pitch.abs() < 0.001);
        assert!((aim.distance - 5.0).abs() < 0.001);
    }
}

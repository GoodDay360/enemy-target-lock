//! First-person view lag while locked.
//!
//! While locked, the player's real aim turns quickly and precisely (so arrows, thrown
//! items and clicks go exactly where they should). The picture doesn't jump with it:
//! the first-person camera eases after the real aim a moment later, so it still feels
//! smooth. When the lock ends the lag fades out and the camera is the player's view again.
//!
//! Pure math only; `lib.rs` applies it in the `camera_pose` hook.

use crate::camera;
use crate::config;
use crate::smooth;

/// How quickly the lag fades in when locking and out after releasing.
const BLEND_RESPONSE: f32 = 6.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct ViewLag {
    /// The drawn view's yaw and pitch, trailing the real aim.
    angles: Option<[f32; 2]>,
    /// 0 = the real aim exactly, 1 = fully the trailing view.
    blend: f32,
}

impl ViewLag {
    /// Advance one frame of `seconds`. `aim` is the player's real yaw and pitch this frame;
    /// returns the yaw and pitch to draw from.
    pub fn update(&mut self, active: bool, aim: [f32; 2], seconds: f32) -> [f32; 2] {
        let goal = if active { 1.0 } else { 0.0 };
        self.blend += (goal - self.blend) * smooth::ease_fraction(BLEND_RESPONSE, seconds);
        if !active && self.blend < 0.001 {
            *self = Self::default();
            return aim;
        }

        let mut angles = self.angles.unwrap_or(aim);
        let turn = camera::angle_between(angles[0], angles[1], aim[0], aim[1])
            * smooth::ease_fraction(config::get().view_speed, seconds);
        if let Some(turned) = camera::step_toward(angles[0], angles[1], aim[0], aim[1], turn) {
            angles = turned;
        }
        self.angles = Some(angles);

        [
            aim[0] + camera::yaw_delta(aim[0], angles[0]) * self.blend,
            aim[1] + (angles[1] - aim[1]) * self.blend,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    #[test]
    fn trails_a_sudden_turn_then_catches_up() {
        let mut lag = ViewLag::default();
        for _ in 0..120 {
            lag.update(true, [0.0, 0.0], FRAME);
        }
        // The real aim jumps 20°: the view only moves part of the way this frame...
        let drawn = lag.update(true, [20.0, 0.0], FRAME);
        assert!(drawn[0] > 0.0 && drawn[0] < 5.0, "{drawn:?}");
        // ...and settles on it after a second.
        let mut drawn = drawn;
        for _ in 0..60 {
            drawn = lag.update(true, [20.0, 0.0], FRAME);
        }
        assert!((drawn[0] - 20.0).abs() < 0.1, "{drawn:?}");
    }

    #[test]
    fn fades_out_to_the_real_aim_after_release() {
        let mut lag = ViewLag::default();
        lag.update(true, [0.0, 0.0], FRAME);
        lag.update(true, [30.0, 10.0], FRAME);
        for _ in 0..240 {
            lag.update(false, [30.0, 10.0], FRAME);
        }
        assert_eq!(lag.update(false, [30.0, 10.0], FRAME), [30.0, 10.0]);
    }

    #[test]
    fn turns_the_short_way_around() {
        let mut lag = ViewLag::default();
        for _ in 0..120 {
            lag.update(true, [179.0, 0.0], FRAME);
        }
        let drawn = lag.update(true, [-179.0, 0.0], FRAME);
        assert!(drawn[0] > 178.0 || drawn[0] < -178.0, "{drawn:?}");
    }
}

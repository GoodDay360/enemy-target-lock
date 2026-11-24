//! Smooth camera motion while locked.
//!
//! The game moves entities 20 times a second, but draws many more frames than that.
//! Turning the camera once per tick makes it jump in visible steps, so instead:
//! - each tick, [`Track`] records the player's eye and the target's aim point;
//! - each frame, it blends the last two records by how far into the tick the frame is,
//!   giving a position that moves as smoothly as the game's own rendering;
//! - [`ease_toward_at`] turns the player's aim a frame-rate independent fraction of the
//!   way there, so it settles on the target without overshooting.

use std::time::Instant;

use crate::camera;

/// How quickly the player's real aim turns to the target while locked (per second).
/// Fast, so shots and clicks go where they should; what's drawn trails it smoothly
/// (`view_lag.rs` in first person, `cinematic.rs` in third person).
pub const AIM_RESPONSE_PER_SECOND: f32 = 20.0;
/// The fastest the camera turns, in degrees per second, so big swings stay readable.
const MAX_TURN_DEGREES_PER_SECOND: f32 = 540.0;
/// Remaining error below this (degrees) is left alone, so the view rests instead of trembling.
const DEAD_ZONE_DEGREES: f32 = 0.05;

/// The last two per-tick positions of the player's eye and the target's aim point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Track {
    previous: Option<Sample>,
    current: Option<Sample>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Sample {
    eye: [f64; 3],
    point: [f64; 3],
}

impl Track {
    /// Record this tick's positions.
    pub fn push(&mut self, eye: [f64; 3], point: [f64; 3]) {
        let sample = Sample { eye, point };
        self.previous = Some(self.current.unwrap_or(sample));
        self.current = Some(sample);
    }

    /// How far the target's aim point moved in the last tick (blocks per tick).
    pub fn target_velocity(&self) -> [f64; 3] {
        match (self.previous, self.current) {
            (Some(previous), Some(current)) => [
                current.point[0] - previous.point[0],
                current.point[1] - previous.point[1],
                current.point[2] - previous.point[2],
            ],
            _ => [0.0; 3],
        }
    }

    /// The eye and aim point `partial_tick` (0..1) of the way from the previous tick to the current one.
    pub fn at(&self, partial_tick: f32) -> Option<([f64; 3], [f64; 3])> {
        let (previous, current) = (self.previous?, self.current?);
        let t = f64::from(partial_tick.clamp(0.0, 1.0));
        Some((lerp(previous.eye, current.eye, t), lerp(previous.point, current.point, t)))
    }
}

/// Turn from `(yaw, pitch)` toward the target over a frame lasting `seconds`, catching
/// up at `response` per second. Returns `None` when the view is already close enough
/// to leave alone.
pub fn ease_toward_at(response: f32, yaw: f32, pitch: f32, target_yaw: f32, target_pitch: f32, seconds: f32) -> Option<[f32; 2]> {
    let error = camera::angle_between(yaw, pitch, target_yaw, target_pitch);
    if !error.is_finite() || error < DEAD_ZONE_DEGREES || !(seconds > 0.0) {
        return None;
    }
    let eased = error * ease_fraction(response, seconds);
    let step = eased.min(MAX_TURN_DEGREES_PER_SECOND * seconds);
    camera::step_toward(yaw, pitch, target_yaw, target_pitch, step)
}

/// Measures the time between frames. Long gaps (a pause, a lag spike) count as at most
/// `MAX_FRAME_SECONDS`, so the camera doesn't jump when the game resumes.
#[derive(Debug, Default)]
pub struct FrameClock {
    last: Option<Instant>,
}

const MAX_FRAME_SECONDS: f32 = 0.1;

impl FrameClock {
    /// Seconds since the previous call (0 on the first call).
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let seconds = self.last.map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        self.last = Some(now);
        seconds.min(MAX_FRAME_SECONDS)
    }
}

/// The share of the remaining gap to close in a frame of `seconds`, catching up at
/// `response` per second. Frame-rate independent: two half frames equal one whole one.
pub fn ease_fraction(response: f32, seconds: f32) -> f32 {
    if seconds > 0.0 { 1.0 - (-response * seconds).exp() } else { 0.0 }
}

/// `t` (0..1) of the way from `a` to `b`.
pub fn lerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_blends_between_the_last_two_ticks() {
        let mut track = Track::default();
        assert_eq!(track.at(0.5), None);
        track.push([0.0; 3], [10.0, 0.0, 0.0]);
        assert_eq!(track.at(0.5), Some(([0.0; 3], [10.0, 0.0, 0.0])));
        track.push([2.0, 0.0, 0.0], [20.0, 0.0, 0.0]);
        assert_eq!(track.at(0.5), Some(([1.0, 0.0, 0.0], [15.0, 0.0, 0.0])));
        assert_eq!(track.at(1.0), Some(([2.0, 0.0, 0.0], [20.0, 0.0, 0.0])));
        assert_eq!(track.target_velocity(), [10.0, 0.0, 0.0]);
    }

    #[test]
    fn easing_is_frame_rate_independent() {
        // One 1/30 s frame should end up where two 1/60 s frames do.
        let one = ease_toward_at(AIM_RESPONSE_PER_SECOND, 0.0, 0.0, 10.0, 0.0, 1.0 / 30.0).unwrap();
        let half = ease_toward_at(AIM_RESPONSE_PER_SECOND, 0.0, 0.0, 10.0, 0.0, 1.0 / 60.0).unwrap();
        let two = ease_toward_at(AIM_RESPONSE_PER_SECOND, half[0], half[1], 10.0, 0.0, 1.0 / 60.0).unwrap();
        assert!((one[0] - two[0]).abs() < 0.01, "{one:?} vs {two:?}");
    }

    #[test]
    fn easing_never_overshoots_and_rests_when_close() {
        let step = ease_toward_at(AIM_RESPONSE_PER_SECOND, 0.0, 0.0, 10.0, 0.0, 10.0).unwrap();
        assert!(step[0] <= 10.0 + 1e-4);
        assert_eq!(ease_toward_at(AIM_RESPONSE_PER_SECOND, 0.0, 0.0, 0.01, 0.0, 1.0 / 60.0), None);
    }

    #[test]
    fn big_swings_are_speed_limited() {
        let step = ease_toward_at(AIM_RESPONSE_PER_SECOND, 0.0, 0.0, 170.0, 0.0, 0.1).unwrap();
        assert!((step[0] - MAX_TURN_DEGREES_PER_SECOND * 0.1).abs() < 0.01);
    }
}

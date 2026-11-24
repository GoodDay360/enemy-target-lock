//! The third-person lock camera: a cinematic over-the-shoulder view that frames the
//! player and the target, drifting slowly into place while the player itself turns
//! to the target right away.
//!
//! Pure math only. `lib.rs` feeds it positions and the frame time in `camera_pose`,
//! and `scan::clip_camera` keeps it out of walls.

use crate::camera;
use crate::smooth;

/// How far behind the player the camera sits (blocks).
const DISTANCE: f64 = 3.6;
/// How far to the right of the player (over the right shoulder).
const SHOULDER: f64 = 0.9;
/// How far above the player's eyes.
const HEIGHT: f64 = 0.35;
/// Where the camera looks along the line from the player (0) to the target (1).
const FOCUS: f64 = 0.55;

/// Field of view while the cinematic camera is fully in, as a share of the normal FOV.
pub const FOV_SCALE: f32 = 0.85;

/// How quickly (per second) the camera's position, its direction, and the blend
/// between it and the normal camera catch up. Lower is slower and smoother.
const POSITION_RESPONSE: f32 = 5.0;
const ANGLE_RESPONSE: f32 = 6.0;
const BLEND_RESPONSE: f32 = 5.0;

/// A camera placement: position, yaw and pitch in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}

/// Where the camera wants to be: behind the player on the side away from the target,
/// over the right shoulder, looking at a point between the two.
pub fn desired_pose(eye: [f64; 3], target: [f64; 3]) -> Option<Pose> {
    let away = [eye[0] - target[0], eye[2] - target[2]];
    let length = away[0].hypot(away[1]);
    if !length.is_finite() || length < 1e-6 {
        return None;
    }
    let back = [away[0] / length, away[1] / length];
    // Looking along -back, the right-hand side is (back.z, -back.x) in Minecraft's axes.
    let right = [back[1], -back[0]];
    let position = [
        eye[0] + back[0] * DISTANCE + right[0] * SHOULDER,
        eye[1] + HEIGHT,
        eye[2] + back[1] * DISTANCE + right[1] * SHOULDER,
    ];
    let focus = smooth::lerp(eye, target, FOCUS);
    let aim = camera::aim_at(position, focus)?;
    Some(Pose { position, yaw: aim.yaw, pitch: aim.pitch })
}

/// The cinematic camera's smoothed state across frames.
#[derive(Clone, Copy, Debug, Default)]
pub struct CinematicCamera {
    /// 0 = the normal camera, 1 = fully the cinematic one.
    blend: f32,
    /// The smoothed cinematic pose, once there is one.
    pose: Option<Pose>,
}

impl CinematicCamera {
    /// How far the cinematic camera is blended in (0..1), for the FOV.
    pub fn blend(&self) -> f32 {
        self.blend
    }

    /// Advance one frame of `seconds` and return the pose to draw.
    /// `desired` is where the cinematic camera wants to be (None while not locked, so it
    /// eases back out), and `normal` is where the game would put the camera.
    pub fn update(&mut self, desired: Option<Pose>, normal: Pose, seconds: f32) -> Pose {
        let blend_goal = if desired.is_some() { 1.0 } else { 0.0 };
        self.blend += (blend_goal - self.blend) * smooth::ease_fraction(BLEND_RESPONSE, seconds);
        if self.blend < 0.001 && desired.is_none() {
            *self = Self::default();
            return normal;
        }

        let goal = desired.or(self.pose).unwrap_or(normal);
        let mut pose = self.pose.unwrap_or(normal);
        pose.position = smooth::lerp(pose.position, goal.position, f64::from(smooth::ease_fraction(POSITION_RESPONSE, seconds)));
        let turn = camera::angle_between(pose.yaw, pose.pitch, goal.yaw, goal.pitch) * smooth::ease_fraction(ANGLE_RESPONSE, seconds);
        if let Some([yaw, pitch]) = camera::step_toward(pose.yaw, pose.pitch, goal.yaw, goal.pitch, turn) {
            pose.yaw = yaw;
            pose.pitch = pitch;
        }
        self.pose = Some(pose);

        blend_poses(normal, pose, self.blend)
    }
}

/// `t` (0..1) of the way from pose `a` to pose `b`, turning the short way around.
fn blend_poses(a: Pose, b: Pose, t: f32) -> Pose {
    Pose {
        position: smooth::lerp(a.position, b.position, f64::from(t)),
        yaw: a.yaw + camera::yaw_delta(a.yaw, b.yaw) * t,
        pitch: a.pitch + (b.pitch - a.pitch) * t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EYE: [f64; 3] = [0.0, 1.6, 0.0];
    const TARGET: [f64; 3] = [0.0, 1.6, 10.0];

    #[test]
    fn sits_behind_the_player_over_the_right_shoulder_facing_the_target() {
        // Target due south (+z): behind is -z; facing south, the right-hand side is -x (west).
        let pose = desired_pose(EYE, TARGET).unwrap();
        assert!((pose.position[2] - -DISTANCE).abs() < 1e-9);
        assert!((pose.position[0] - -SHOULDER).abs() < 1e-9);
        assert!((pose.position[1] - (EYE[1] + HEIGHT)).abs() < 1e-9);
        assert!(pose.yaw.abs() < 10.0, "roughly facing south, got {}", pose.yaw);
    }

    #[test]
    fn no_pose_when_the_target_is_straight_above() {
        assert_eq!(desired_pose(EYE, [0.0, 10.0, 0.0]), None);
    }

    #[test]
    fn eases_in_then_back_out_to_the_normal_camera() {
        let normal = Pose { position: [0.0, 1.6, 0.0], yaw: 0.0, pitch: 0.0 };
        let desired = desired_pose(EYE, TARGET);
        let mut cam = CinematicCamera::default();

        let first = cam.update(desired, normal, 1.0 / 60.0);
        assert!(cam.blend() > 0.0 && cam.blend() < 0.2, "starts gently: {}", cam.blend());
        assert_ne!(first, normal);

        for _ in 0..240 {
            cam.update(desired, normal, 1.0 / 60.0);
        }
        assert!(cam.blend() > 0.99);

        for _ in 0..240 {
            cam.update(None, normal, 1.0 / 60.0);
        }
        assert_eq!(cam.update(None, normal, 1.0 / 60.0), normal);
        assert_eq!(cam.blend(), 0.0);
    }
}

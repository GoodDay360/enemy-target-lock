//! The third-person lock camera: a cinematic over-the-shoulder view that frames the
//! player and the target, drifting slowly into place while the player itself turns
//! to the target right away.
//!
//! Pure math only. `lib.rs` feeds it positions and the frame time in `camera_pose`,
//! and `scan::clip_camera` keeps it out of walls.
//!
//! Where it sits, where it looks, its FOV and how quickly it moves come from the
//! `[third_person]` config section.

use crate::camera;
use crate::config;
use crate::smooth;

/// How quickly (per second) the blend between the normal camera and the cinematic one
/// catches up when locking and releasing.
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
    let config = config::get();
    let away = [eye[0] - target[0], eye[2] - target[2]];
    let length = away[0].hypot(away[1]);
    if !length.is_finite() || length < 1e-6 {
        return None;
    }
    let back = [away[0] / length, away[1] / length];
    // Looking along -back, the right-hand side is (back.z, -back.x) in Minecraft's axes.
    let right = [back[1], -back[0]];
    let position = [
        eye[0] + back[0] * config.camera_distance + right[0] * config.camera_shoulder,
        eye[1] + config.camera_height,
        eye[2] + back[1] * config.camera_distance + right[1] * config.camera_shoulder,
    ];
    let focus = smooth::lerp(eye, target, config.camera_focus);
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

        let config = config::get();
        let goal = desired.or(self.pose).unwrap_or(normal);
        let mut pose = self.pose.unwrap_or(normal);
        pose.position = smooth::lerp(pose.position, goal.position, f64::from(smooth::ease_fraction(config.camera_position_speed, seconds)));
        let turn = camera::angle_between(pose.yaw, pose.pitch, goal.yaw, goal.pitch) * smooth::ease_fraction(config.camera_angle_speed, seconds);
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

    use crate::config::Config;

    const EYE: [f64; 3] = [0.0, 1.6, 0.0];
    const TARGET: [f64; 3] = [0.0, 1.6, 10.0];

    #[test]
    fn sits_behind_the_player_over_the_right_shoulder_facing_the_target() {
        // Target due south (+z): behind is -z; facing south, the right-hand side is -x (west).
        let pose = desired_pose(EYE, TARGET).unwrap();
        let defaults = Config::DEFAULT;
        assert!((pose.position[2] - -defaults.camera_distance).abs() < 1e-9);
        assert!((pose.position[0] - -defaults.camera_shoulder).abs() < 1e-9);
        assert!((pose.position[1] - (EYE[1] + defaults.camera_height)).abs() < 1e-9);
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

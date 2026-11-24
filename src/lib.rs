//! Subtle client-side aim correction for hostile mobs or players under the crosshair.
use mod_api::{self, ClientHost, CrosshairTarget, Hooks};

const MAX_ANGLE_DEGREES: f32 = 8.0;
const MAX_STEP_DEGREES: f32 = 0.25;
const ASSISTED_TYPES: &[&str] = &[
    "minecraft:blaze",
    "minecraft:cave_spider",
    "minecraft:creeper",
    "minecraft:drowned",
    "minecraft:elder_guardian",
    "minecraft:ender_dragon",
    "minecraft:endermite",
    "minecraft:evoker",
    "minecraft:ghast",
    "minecraft:guardian",
    "minecraft:hoglin",
    "minecraft:husk",
    "minecraft:magma_cube",
    "minecraft:phantom",
    "minecraft:piglin_brute",
    "minecraft:pillager",
    "minecraft:ravager",
    "minecraft:shulker",
    "minecraft:silverfish",
    "minecraft:skeleton",
    "minecraft:slime",
    "minecraft:stray",
    "minecraft:vex",
    "minecraft:vindicator",
    "minecraft:witch",
    "minecraft:wither",
    "minecraft:wither_skeleton",
    "minecraft:zoglin",
    "minecraft:zombie",
    "minecraft:zombie_villager",
    "minecraft:zombified_piglin",
    "minecraft:player",
];

fn correction_step(yaw: f32, pitch: f32, target_yaw: f32, target_pitch: f32) -> Option<[f32; 2]> {
    let yaw_delta = (target_yaw - yaw + 180.0).rem_euclid(360.0) - 180.0;
    let pitch_delta = target_pitch - pitch;
    let angle = yaw_delta.hypot(pitch_delta);
    if !angle.is_finite() || angle > MAX_ANGLE_DEGREES || angle < 0.01 {
        return None;
    }

    let scale = angle.min(MAX_STEP_DEGREES) / angle;
    Some([yaw + yaw_delta * scale, pitch + pitch_delta * scale])
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn client_tick_pre(&self, host: &mut dyn ClientHost, _event: &mut mod_api::ClientTickPreEvent) {
        if !host.playing() {
            return;
        }

        let Some(CrosshairTarget::Entity { entity, .. }) = host.crosshair_target() else {
            return;
        };
        let Some(kind) = entity.kind(host) else {
            return;
        };
        if !ASSISTED_TYPES.contains(&kind.id(host).as_str()) {
            return;
        }

        let Some(player) = host.camera_entity() else {
            return;
        };
        if player.kind(host).map(|kind| kind.id(host)).as_deref() != Some("minecraft:player") {
            return;
        }
        let (Some([yaw, pitch]), Some(eye), Some(bounds)) = (
            player.rotation(host),
            player.eye_position(host),
            entity.bounding_box(host),
        ) else {
            return;
        };

        let center = [
            (bounds[0][0] + bounds[1][0]) / 2.0,
            (bounds[0][1] + bounds[1][1]) / 2.0,
            (bounds[0][2] + bounds[1][2]) / 2.0,
        ];
        let dx = center[0] - eye[0];
        let dy = center[1] - eye[1];
        let dz = center[2] - eye[2];
        let horizontal = dx.hypot(dz);
        if !horizontal.is_finite() || horizontal == 0.0 {
            return;
        }

        let target_yaw = (-dx).atan2(dz).to_degrees() as f32;
        let target_pitch = -dy.atan2(horizontal).to_degrees() as f32;
        if let Some([yaw, pitch]) = correction_step(yaw, pitch, target_yaw, target_pitch) {
            let _ = host.set_local_rotation(yaw, pitch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_delta_uses_shortest_path_across_wraparound() {
        let step = correction_step(-179.0, 0.0, 179.0, 0.0).unwrap();
        assert!((step[0] - -179.25).abs() < 0.001);
    }

    #[test]
    fn correction_is_limited_and_stops_outside_the_assist_cone() {
        let step = correction_step(0.0, 0.0, 5.0, 0.0).unwrap();
        assert!((step[0] - MAX_STEP_DEGREES).abs() < 0.001);
        assert!(correction_step(0.0, 0.0, 9.0, 0.0).is_none());
    }

    #[test]
    fn hostile_mobs_and_players_are_assisted_but_passive_mobs_are_not() {
        assert!(ASSISTED_TYPES.contains(&"minecraft:zombie"));
        assert!(ASSISTED_TYPES.contains(&"minecraft:player"));
        assert!(!ASSISTED_TYPES.contains(&"minecraft:cow"));
    }
}

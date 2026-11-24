//! Choosing a target: which entities count, which one to lock first, and which
//! one to switch to. Pure logic only; the world lookups live in `scan.rs`.
//!
//! On the client the API can't tell whether another entity is "living", so
//! mobs are recognised by leaving out known non-mob type ids.

use crate::camera;
use crate::config;
use crate::input::Side;

/// How much one block of distance counts against a target, in degrees of off-center.
const DISTANCE_WEIGHT: f32 = 0.5;

/// Exact entity type ids that are never locked onto.
const NON_MOB_TYPES: &[&str] = &[
    "minecraft:area_effect_cloud",
    "minecraft:armor_stand",
    "minecraft:egg",
    "minecraft:end_crystal",
    "minecraft:experience_bottle",
    "minecraft:experience_orb",
    "minecraft:eye_of_ender",
    "minecraft:falling_block",
    "minecraft:glow_item_frame",
    "minecraft:interaction",
    "minecraft:item",
    "minecraft:item_frame",
    "minecraft:leash_knot",
    "minecraft:lightning_bolt",
    "minecraft:marker",
    "minecraft:ominous_item_spawner",
    "minecraft:painting",
    "minecraft:tnt",
    "minecraft:block_display",
    "minecraft:item_display",
    "minecraft:text_display",
];

/// Type id suffixes that mark whole families of non-mob entities
/// (`oak_boat`, `chest_minecart`, `spectral_arrow`, `ender_pearl`, ...).
const NON_MOB_SUFFIXES: &[&str] = &[
    "_boat", "_raft", "minecart", "arrow", "trident", "fireball", "_pearl", "_potion", "snowball",
    "_skull", "firework_rocket", "fishing_bobber", "llama_spit", "shulker_bullet", "evoker_fangs",
    "wind_charge",
];

/// Whether an entity type id (e.g. `minecraft:zombie`) can be locked onto.
/// Players only when `include_players` is on.
pub fn is_lockable(type_id: &str) -> bool {
    if type_id == "minecraft:player" {
        return config::get().include_players;
    }
    !NON_MOB_TYPES.contains(&type_id) && !NON_MOB_SUFFIXES.iter().any(|suffix| type_id.ends_with(suffix))
}

/// A possible target as seen from the player's eye.
#[derive(Clone, Copy, Debug)]
pub struct Candidate<T> {
    pub target: T,
    pub aim: camera::Aim,
    pub visible: bool,
}

/// The target a new lock should pick: visible, in range, inside the pick cone,
/// preferring the one nearest the center of the view, then the closest.
pub fn pick_best<T: Copy>(yaw: f32, pitch: f32, candidates: &[Candidate<T>]) -> Option<T> {
    let config = config::get();
    candidates
        .iter()
        .filter(|c| c.visible && c.aim.distance <= config.lock_range)
        .map(|c| (c, camera::angle_between(yaw, pitch, c.aim.yaw, c.aim.pitch)))
        .filter(|(_, off_center)| *off_center <= config.pick_cone_degrees)
        .map(|(c, off_center)| (c.target, off_center + c.aim.distance as f32 * DISTANCE_WEIGHT))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(target, _)| target)
}

/// The target to switch to: the nearest visible one on `side` of the current target's
/// direction (`current_yaw`), within 90°. `None` when there is nothing on that side.
pub fn pick_next<T: Copy>(current_yaw: f32, side: Side, candidates: &[Candidate<T>]) -> Option<T> {
    let range = config::get().lock_range;
    candidates
        .iter()
        .filter(|c| c.visible && c.aim.distance <= range)
        .map(|c| (c.target, camera::yaw_delta(current_yaw, c.aim.yaw)))
        .filter(|(_, delta)| match side {
            Side::Right => *delta > 1.0 && *delta <= 90.0,
            Side::Left => *delta < -1.0 && *delta >= -90.0,
        })
        .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map(|(target, _)| target)
}

/// The target to move on to when the locked one dies: the closest visible one in range,
/// in any direction.
pub fn pick_nearest<T: Copy>(candidates: &[Candidate<T>]) -> Option<T> {
    let range = config::get().lock_range;
    candidates
        .iter()
        .filter(|c| c.visible && c.aim.distance <= range)
        .min_by(|a, b| a.aim.distance.total_cmp(&b.aim.distance))
        .map(|c| c.target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: u32, yaw: f32, distance: f64, visible: bool) -> Candidate<u32> {
        Candidate { target: id, aim: camera::Aim { yaw, pitch: 0.0, distance }, visible }
    }

    #[test]
    fn all_mobs_and_players_are_lockable() {
        for id in ["minecraft:zombie", "minecraft:cow", "minecraft:villager", "minecraft:player", "minecraft:ender_dragon", "othermod:custom_mob"] {
            assert!(is_lockable(id), "{id}");
        }
    }

    #[test]
    fn objects_and_projectiles_are_not_lockable() {
        for id in ["minecraft:oak_boat", "minecraft:chest_minecart", "minecraft:item_frame", "minecraft:armor_stand", "minecraft:arrow", "minecraft:item"] {
            assert!(!is_lockable(id), "{id}");
        }
    }

    #[test]
    fn picks_the_target_nearest_the_view_center() {
        let candidates = [candidate(1, 30.0, 5.0, true), candidate(2, 5.0, 8.0, true), candidate(3, -40.0, 3.0, true)];
        assert_eq!(pick_best(0.0, 0.0, &candidates), Some(2));
    }

    #[test]
    fn pick_skips_hidden_far_and_behind_targets() {
        let candidates = [candidate(1, 0.0, 5.0, false), candidate(2, 0.0, 30.0, true), candidate(3, 170.0, 3.0, true)];
        assert_eq!(pick_best(0.0, 0.0, &candidates), None);
    }

    #[test]
    fn after_a_kill_the_nearest_visible_target_is_next_in_any_direction() {
        let candidates = [candidate(1, 0.0, 10.0, true), candidate(2, 170.0, 4.0, true), candidate(3, 10.0, 2.0, false), candidate(4, 0.0, 30.0, true)];
        assert_eq!(pick_nearest(&candidates), Some(2));
        assert_eq!(pick_nearest(&candidates[2..]), None);
    }

    #[test]
    fn switching_goes_to_the_nearest_target_on_that_side() {
        let candidates = [candidate(1, 0.0, 5.0, true), candidate(2, 20.0, 5.0, true), candidate(3, 50.0, 5.0, true), candidate(4, -30.0, 5.0, true)];
        assert_eq!(pick_next(0.0, Side::Right, &candidates), Some(2));
        assert_eq!(pick_next(0.0, Side::Left, &candidates), Some(4));
        assert_eq!(pick_next(50.0, Side::Right, &candidates), None);
    }
}

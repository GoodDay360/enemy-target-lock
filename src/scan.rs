//! World lookups: the player's view, nearby mobs, and line of sight.

use mod_api::{ClientHost, ClipBlock, ClipFluid, EntityRef, Hand, LevelRef};

use crate::ballistics::{self, Projectile};

use crate::camera::{self, Aim};
use crate::targeting::{self, Candidate, LOCK_RANGE};

/// The local player's eye position and view direction.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub player: EntityRef,
    pub eye: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}

/// The local player's view, or `None` when the camera isn't on the player
/// (spectating another entity, no world open).
pub fn player_view(host: &dyn ClientHost) -> Option<View> {
    let player = host.camera_entity()?;
    if player.kind(host).map(|kind| kind.id(host)).as_deref() != Some("minecraft:player") {
        return None;
    }
    let [yaw, pitch] = player.rotation(host)?;
    let eye = player.eye_position(host)?;
    Some(View { player, eye, yaw, pitch })
}

/// Whether the view is third person from behind (F5 once), where the cinematic lock
/// camera is used.
pub fn third_person_behind(host: &dyn ClientHost) -> bool {
    host.camera_type() == mod_api::CameraType::ThirdPersonBack
}

/// The projectile the player would launch right now, if they hold a projectile item:
/// the item being used (a bow being drawn), else the main hand's, else the off hand's.
pub fn held_projectile(host: &dyn ClientHost, view: &View) -> Option<Projectile> {
    let living = host.living(view.player)?;
    let projectile_in = |hand: Hand| {
        let stack = living.hand_item(host, hand)?;
        ballistics::projectile_for(stack.item().id(host).as_str())
    };
    match host.local_item_use() {
        Some(item_use) => projectile_in(item_use.hand),
        None => projectile_in(Hand::Main).or_else(|| projectile_in(Hand::Off)),
    }
}

/// Where a projectile leaves the player: just below the eyes, as in the game.
pub fn launch_point(eye: [f64; 3]) -> [f64; 3] {
    [eye[0], eye[1] - 0.1, eye[2]]
}

/// Whether the view is first person, where the trailing view (`view_lag.rs`) is used.
pub fn first_person(host: &dyn ClientHost) -> bool {
    host.camera_type() == mod_api::CameraType::FirstPerson
}

/// The level the client is showing.
fn client_level(host: &dyn ClientHost) -> Option<LevelRef> {
    host.level(host.dimension())
}

/// The aim at `target` from the player's eye, or `None` if the target is gone.
pub fn aim_at_entity(host: &dyn ClientHost, view: &View, target: EntityRef) -> Option<Aim> {
    let bounds = target.bounding_box(host)?;
    camera::aim_at(view.eye, camera::aim_point(bounds))
}

/// Whether nothing solid blocks the straight line from the eye to `aim`'s point.
/// Assumes visible when the world can't be read, so a failed lookup never drops a lock.
pub fn has_line_of_sight(host: &dyn ClientHost, view: &View, target: EntityRef) -> bool {
    let (Some(level), Some(bounds)) = (client_level(host), target.bounding_box(host)) else {
        return true;
    };
    let point = camera::aim_point(bounds);
    let Some(hit) = level.clip(host, view.eye, point, ClipBlock::Collider, ClipFluid::None) else {
        return true;
    };
    if hit.miss {
        return true;
    }
    let to_hit = distance(view.eye, hit.location);
    let to_target = distance(view.eye, point);
    to_hit + 0.3 >= to_target
}

/// Every lockable mob or player within `LOCK_RANGE` of the player, with its aim and visibility.
pub fn nearby_candidates(host: &dyn ClientHost, view: &View) -> Vec<Candidate<EntityRef>> {
    let Some(level) = client_level(host) else {
        return Vec::new();
    };
    let min = view.eye.map(|v| v - LOCK_RANGE);
    let max = view.eye.map(|v| v + LOCK_RANGE);
    let Some(entities) = level.entities_in(host, min, max) else {
        return Vec::new();
    };

    entities
        .into_iter()
        .filter(|&entity| entity != view.player)
        .filter(|entity| entity.kind(host).is_some_and(|kind| targeting::is_lockable(kind.id(host).as_str())))
        .filter_map(|entity| {
            let aim = aim_at_entity(host, view, entity)?;
            let visible = has_line_of_sight(host, view, entity);
            Some(Candidate { target: entity, aim, visible })
        })
        .collect()
}

/// Pull a camera position in from `from` (the player's eye) so it stays in front of any
/// wall between the two, as the normal third-person camera does.
pub fn clip_camera(host: &dyn ClientHost, from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
    const MARGIN: f64 = 0.2;
    let Some(hit) = client_level(host).and_then(|level| level.clip(host, from, to, ClipBlock::Collider, ClipFluid::None)) else {
        return to;
    };
    if hit.miss {
        return to;
    }
    let full = distance(from, to);
    if full < 1e-6 {
        return to;
    }
    let allowed = (distance(from, hit.location) - MARGIN).max(0.0);
    let t = allowed / full;
    [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t, from[2] + (to[2] - from[2]) * t]
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

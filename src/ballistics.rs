//! Aiming projectiles: arrows, tridents, snowballs, eggs, ender pearls, potions...
//!
//! A projectile doesn't fly in a straight line: it drops with gravity, slows down
//! with drag, and takes time to arrive. Aiming straight at the target makes it land
//! short and behind a moving target. So for a held projectile item this module:
//! 1. knows how the game launches it (speed, gravity, drag, angle offset);
//! 2. simulates its flight tick by tick, as the game does, and searches for the
//!    launch angle that lands it on the target;
//! 3. leads a moving target by where it will be when the projectile arrives.
//!
//! Pure math only; `scan::held_projectile` reads the held item.

/// How the game launches and moves one kind of projectile (blocks and ticks).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projectile {
    /// Launch speed, blocks per tick.
    pub speed: f64,
    /// Speed lost downward each tick.
    pub gravity: f64,
    /// Velocity kept each tick (air resistance).
    pub drag: f64,
    /// Degrees the game adds to the player's pitch at launch (potions are thrown 20° higher).
    pub pitch_offset: f32,
}

impl Projectile {
    const fn new(speed: f64, gravity: f64, drag: f64, pitch_offset: f32) -> Self {
        Self { speed, gravity, drag, pitch_offset }
    }
}

/// The projectile thrown or shot by `item_id`, if it is a projectile item.
///
/// A bow is always aimed for a full draw (its fastest arrow). Following the draw as it
/// builds up would raise the aim sharply when drawing starts (a barely drawn arrow is
/// slow and drops a lot) and lower it again as the draw fills, which looks like the
/// player tilting up and back. Shots are nearly always released at full draw anyway.
pub fn projectile_for(item_id: &str) -> Option<Projectile> {
    let projectile = match item_id {
        "minecraft:bow" => Projectile::new(3.0, 0.05, 0.99, 0.0),
        "minecraft:crossbow" => Projectile::new(3.15, 0.05, 0.99, 0.0),
        "minecraft:trident" => Projectile::new(2.5, 0.05, 0.99, 0.0),
        "minecraft:snowball" | "minecraft:egg" | "minecraft:ender_pearl" => Projectile::new(1.5, 0.03, 0.99, 0.0),
        "minecraft:splash_potion" | "minecraft:lingering_potion" => Projectile::new(0.5, 0.05, 0.99, -20.0),
        "minecraft:experience_bottle" => Projectile::new(0.7, 0.07, 0.99, -20.0),
        "minecraft:wind_charge" => Projectile::new(1.5, 0.0, 1.0, 0.0),
        _ => return None,
    };
    Some(projectile)
}

/// The aim that lands `projectile` on `target`, shot from `from`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solution {
    /// The player's yaw and pitch to use (pitch already corrected for `pitch_offset`).
    pub yaw: f32,
    pub pitch: f32,
    /// Ticks until it arrives.
    pub flight_ticks: f64,
}

/// Steepest launch angle searched, in degrees above level. Beyond about this the
/// projectile only goes higher, not farther, so it's a lob rather than a shot.
const MAX_ELEVATION: f64 = 45.0;
const MIN_ELEVATION: f64 = -85.0;
/// Flights longer than this are treated as out of range.
const MAX_FLIGHT_TICKS: u32 = 200;

/// Aim at `target` (whose velocity is `target_velocity` blocks per tick) from `from`.
/// `None` if the target is out of the projectile's range.
pub fn solve(projectile: Projectile, from: [f64; 3], target: [f64; 3], target_velocity: [f64; 3]) -> Option<Solution> {
    // Lead the target: aim where it will be after the flight, refining a few times
    // since a farther point takes longer to reach.
    let mut aim_point = target;
    let mut solution = None;
    for _ in 0..4 {
        let found = solve_static(projectile, from, aim_point)?;
        aim_point = [
            target[0] + target_velocity[0] * found.flight_ticks,
            target[1] + target_velocity[1] * found.flight_ticks,
            target[2] + target_velocity[2] * found.flight_ticks,
        ];
        solution = Some(found);
    }
    solution
}

/// Aim at a point that isn't moving.
fn solve_static(projectile: Projectile, from: [f64; 3], target: [f64; 3]) -> Option<Solution> {
    let dx = target[0] - from[0];
    let dz = target[2] - from[2];
    let horizontal = dx.hypot(dz);
    let rise = target[1] - from[1];
    if !horizontal.is_finite() || horizontal < 1e-6 || projectile.speed <= 0.0 {
        return None;
    }

    // Height reached at the target's distance grows with the launch angle (for the flat,
    // direct shots searched here), so a bisection finds the angle that matches `rise`.
    let (mut low, mut high) = (MIN_ELEVATION, MAX_ELEVATION);
    let height_at = |elevation: f64| flight(projectile, elevation, horizontal).map(|(height, _)| height);
    if height_at(high).is_none_or(|height| height < rise) {
        return None;
    }
    for _ in 0..50 {
        let middle = (low + high) / 2.0;
        match height_at(middle) {
            Some(height) if height >= rise => high = middle,
            _ => low = middle,
        }
    }
    let (_, flight_ticks) = flight(projectile, high, horizontal)?;
    let yaw = (-dx).atan2(dz).to_degrees() as f32;
    // Minecraft's pitch is negative looking up; the game adds `pitch_offset` at launch.
    let pitch = (-high) as f32 - projectile.pitch_offset;
    Some(Solution { yaw, pitch, flight_ticks })
}

/// Fly a projectile launched `elevation` degrees above level until it has covered
/// `horizontal` blocks. Returns the height it is at there and the (fractional) ticks
/// it took, or `None` if it never gets that far.
fn flight(projectile: Projectile, elevation: f64, horizontal: f64) -> Option<(f64, f64)> {
    let radians = elevation.to_radians();
    let (mut speed_h, mut speed_y) = (projectile.speed * radians.cos(), projectile.speed * radians.sin());
    let (mut x, mut y) = (0.0, 0.0);
    for tick in 0..MAX_FLIGHT_TICKS {
        let (next_x, next_y) = (x + speed_h, y + speed_y);
        if next_x >= horizontal {
            let t = (horizontal - x) / speed_h;
            return Some((y + (next_y - y) * t, f64::from(tick) + t));
        }
        (x, y) = (next_x, next_y);
        speed_h *= projectile.drag;
        speed_y = speed_y * projectile.drag - projectile.gravity;
        if speed_h < 1e-4 {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const FROM: [f64; 3] = [0.0, 0.0, 0.0];

    fn arrow() -> Projectile {
        projectile_for("minecraft:bow").unwrap()
    }

    #[test]
    fn knows_projectile_items_only() {
        assert!(projectile_for("minecraft:snowball").is_some());
        assert!(projectile_for("minecraft:diamond_sword").is_none());
    }

    #[test]
    fn aims_above_a_level_target_to_make_up_for_the_drop() {
        let solution = solve(arrow(), FROM, [0.0, 0.0, 30.0], [0.0; 3]).unwrap();
        assert!(solution.pitch < -0.5, "should aim up, got {}", solution.pitch);
        assert!(solution.yaw.abs() < 0.01);
        // The chosen angle really lands there.
        let (height, _) = flight(arrow(), f64::from(-solution.pitch), 30.0).unwrap();
        assert!(height.abs() < 0.01, "lands {height} off");
    }

    #[test]
    fn slower_projectiles_aim_higher() {
        let arrow_pitch = solve(arrow(), FROM, [0.0, 0.0, 20.0], [0.0; 3]).unwrap().pitch;
        let snowball = projectile_for("minecraft:snowball").unwrap();
        let snowball_pitch = solve(snowball, FROM, [0.0, 0.0, 20.0], [0.0; 3]).unwrap().pitch;
        assert!(snowball_pitch < arrow_pitch);
    }

    #[test]
    fn potions_account_for_their_launch_offset() {
        let potion = projectile_for("minecraft:splash_potion").unwrap();
        let solution = solve(potion, FROM, [0.0, 0.0, 4.0], [0.0; 3]).unwrap();
        // The game throws 20° above the view, so the view itself points lower.
        let (height, _) = flight(potion, f64::from(-(solution.pitch + potion.pitch_offset)), 4.0).unwrap();
        assert!(height.abs() < 0.01);
    }

    #[test]
    fn leads_a_moving_target() {
        let still = solve(arrow(), FROM, [0.0, 0.0, 30.0], [0.0; 3]).unwrap();
        let moving = solve(arrow(), FROM, [0.0, 0.0, 30.0], [-0.2, 0.0, 0.0]).unwrap();
        // Moving toward -x (east to west): aim toward -x, which is a positive yaw.
        assert!(moving.yaw > still.yaw + 1.0, "{} vs {}", moving.yaw, still.yaw);
    }

    #[test]
    fn out_of_range_targets_have_no_solution() {
        let potion = projectile_for("minecraft:splash_potion").unwrap();
        assert!(solve(potion, FROM, [0.0, 0.0, 200.0], [0.0; 3]).is_none());
    }

    #[test]
    fn wind_charges_fly_straight() {
        let wind = projectile_for("minecraft:wind_charge").unwrap();
        let solution = solve(wind, FROM, [0.0, 0.0, 10.0], [0.0; 3]).unwrap();
        assert!(solution.pitch.abs() < 0.01);
    }
}

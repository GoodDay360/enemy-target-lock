//! Clicks go to the locked target.
//!
//! While locked, the game is told to attack (left click) and use items on (right click)
//! the locked target instead of whatever the crosshair ray hits, so the camera never
//! has to snap onto the target for a hit to land. The game still checks reach and line
//! of sight when the click happens.

use mod_api::{ClientHost, EntityRef};

/// Make clicks go to `target`, or back to the crosshair when `None`.
/// Cheap to call every tick: it only changes the setting when it differs.
pub fn direct_clicks_at(host: &dyn ClientHost, target: Option<EntityRef>) {
    let mut targeting = host.targeting();
    if targeting.entity != target {
        targeting.entity = target;
        let _ = host.set_targeting(targeting);
    }
}

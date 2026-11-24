//! Enemy Target Lock: Horizon-native mod entry point.
//!
//! Replace the starter hook with client-side target selection once the target-lock behavior
//! and supported Horizon hook are confirmed.
use mod_api::{self, Hooks, PlayerLoggedInEvent, ServerHost, Text};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Starter hook to verify the native SDK/build pipeline.
    fn player_logged_in(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent<'_>) {
        if let Some(player) = host.player(event.player) {
            let _ = player.tell(
                host,
                Text::literal(format!("Enemy Target Lock loaded for {}.", event.name)),
            );
        }
    }
}

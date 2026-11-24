//! Where targeting is allowed: only where the world's owner allowed it.
//!
//! Lock-on, the aim assist, sending clicks to the target and projectile aiming count
//! as aim assistance, which many servers forbid (and the Horizon mod registry requires
//! such features to stay off on servers that haven't allowed them). So they run:
//!
//! - **in your own world** (singleplayer): always;
//! - **in your own world opened to LAN** with friends in it: when **Allow LAN Friends** is
//!   on, since you are the host allowing it;
//! - **in a friend's LAN world**: when their copy of the mod answered "allowed" to the
//!   consent handshake (`lan.rs`);
//! - **anywhere else** (servers, Realms, hosts without the mod): never.
//!
//! The rest of the mod (settings, HUD) loads everywhere; it just has nothing to do.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use mod_api::ClientHost;

use crate::config;
use crate::lan;

/// Whether targeting may run.
/// - `own_world`: this window runs the world (`integrated_local`);
/// - `players`: how many are listed in it (`None` when unknown, before anyone could join);
/// - `lan_friends`: the host's **Allow LAN Friends** setting;
/// - `host_allows`: what the server this window joined said.
pub fn allowed(own_world: bool, players: Option<usize>, lan_friends: bool, host_allows: bool) -> bool {
    if own_world {
        match players {
            None => false,
            Some(count) if count <= 1 => true,
            Some(_) => lan_friends,
        }
    } else {
        host_allows
    }
}

/// The current answer, updated each tick, plus whether the player has been told why
/// lock-on is off in this session.
#[derive(Default)]
pub struct SessionGate {
    allowed: AtomicBool,
    told: AtomicBool,
    in_world: AtomicBool,
    world_mode: AtomicU32,
}

impl SessionGate {
    /// Each tick: work out whether targeting may run here.
    pub fn update(&self, host: &mut dyn ClientHost) {
        let session = host.client_session();
        let Some(server) = session.client_server() else {
            // Left the world: forget its answer and the notice, so the next one starts fresh.
            if self.in_world.swap(false, Ordering::Relaxed) {
                lan::forget();
                self.told.store(false, Ordering::Relaxed);
            }
            self.allowed.store(false, Ordering::Relaxed);
            return;
        };
        self.in_world.store(true, Ordering::Relaxed);
        let mode = if server.integrated_local { 1 } else { 2 };
        if self.world_mode.swap(mode, Ordering::Relaxed) != mode {
            lan::forget();
            self.told.store(false, Ordering::Relaxed);
        }
        if !server.integrated_local {
            lan::ask_until_answered(host);
        }
        let session = host.client_session();
        let players = session.client_tab_list().map(|list| list.entries.len());
        let allowed = allowed(server.integrated_local, players, config::get().lan_friends, lan::host_allows());
        self.allowed.store(allowed, Ordering::Relaxed);
    }

    pub fn allowed(&self) -> bool {
        self.allowed.load(Ordering::Relaxed)
    }

    /// The player tried to lock where it isn't allowed: tell them why, once a session.
    pub fn explain_once(&self, host: &mut dyn ClientHost) {
        if !self.told.swap(true, Ordering::Relaxed) {
            let line = "Enemy Target Lock: lock-on is off here. It works in your own worlds, and in a \
                        friend's LAN world when they have the mod and allow it.";
            let _ = host.chat_line(mod_api::Text::literal(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn your_own_world() {
        assert!(allowed(true, Some(1), false, false), "singleplayer");
        assert!(!allowed(true, None, false, false), "unknown solo status is disabled");
        assert!(allowed(true, Some(3), true, false), "hosting LAN, friends allowed");
        assert!(!allowed(true, Some(3), false, false), "hosting LAN, friends not allowed");
    }

    #[test]
    fn someone_elses_world() {
        assert!(allowed(false, Some(2), false, true), "a LAN host with the mod allowed it");
        assert!(!allowed(false, Some(2), false, false), "a host without the mod, or one that said no");
        assert!(!allowed(false, Some(1), true, false), "your own setting doesn't count elsewhere");
    }
}

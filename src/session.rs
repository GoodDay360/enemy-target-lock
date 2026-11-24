//! Where targeting is allowed: only where the world's owner allowed it.
//!
//! Lock-on, the aim assist, sending clicks to the target and projectile aiming count
//! as aim assistance, which many servers forbid (and the Horizon mod registry requires
//! such features to stay off on servers that haven't allowed them). So they run:
//!
//! - **in your own integrated world**: always for the integrated-server owner;
//! - **in a friend's LAN world**: when their copy of the mod answered "allowed" to the
//!   consent handshake (`lan.rs`), controlled by the owner's **Allow LAN Friends** setting;
//! - **anywhere else** (servers, Realms, hosts without the mod): never.
//!
//! The rest of the mod (settings, HUD) loads everywhere; it just has nothing to do.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use mod_api::ClientHost;

use crate::config;
use crate::lan;

/// Whether targeting may run.
/// - `own_world`: this window owns the integrated server (`integrated_local`);
/// - `host_allows`: what the server this window joined said.
pub fn allowed(enabled: bool, own_world: bool, host_allows: bool) -> bool {
    if !enabled {
        return false;
    }
    if own_world {
        true
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
    connection: Mutex<Option<String>>,
}

impl SessionGate {
    /// Each tick: work out whether targeting may run here.
    pub fn update(&self, host: &mut dyn ClientHost) {
        if !config::get().enabled {
            lan::forget();
            self.allowed.store(false, Ordering::Relaxed);
            return;
        }
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
        let connection = if server.integrated_local {
            server.save_folder.clone().unwrap_or_default()
        } else {
            server.address.clone().unwrap_or_default()
        };
        let connection_changed = {
            let mut current = self.connection.lock().unwrap_or_else(|p| p.into_inner());
            let changed = current.as_deref() != Some(connection.as_str()) || (connection.is_empty() && current.is_some());
            if changed {
                *current = (!connection.is_empty()).then_some(connection.clone());
            }
            changed
        };
        if connection_changed {
            lan::forget();
            self.told.store(false, Ordering::Relaxed);
        }
        if !server.integrated_local {
            lan::ask_until_answered(host);
        }
        let allowed = allowed(config::get().enabled, server.integrated_local, lan::host_allows());
        self.allowed.store(allowed, Ordering::Relaxed);
    }

    pub fn allowed(&self) -> bool {
        if !config::get().enabled || lan::revoked() {
            self.allowed.store(false, Ordering::Relaxed);
            return false;
        }
        self.allowed.load(Ordering::Relaxed)
    }

    pub fn revoke(&self) {
        lan::forget();
        self.allowed.store(false, Ordering::Relaxed);
        self.in_world.store(false, Ordering::Relaxed);
    }

    /// The player tried to lock where it isn't allowed: tell them why, once a session.
    pub fn explain_once(&self, host: &mut dyn ClientHost) {
        if !config::get().enabled {
            return;
        }
        if !self.told.swap(true, Ordering::Relaxed) {
            let line = "Enemy Target Lock: lock-on is off here. Enable Allow LAN Friends in your \
                        integrated world, or join a friend's LAN world where the host allows it.";
            let _ = host.chat_line(mod_api::Text::literal(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrated_world_owner_is_always_allowed() {
        assert!(allowed(true, true, false), "the owner of the integrated world is trusted");
        assert!(!allowed(false, true, false), "the master switch disables the owner too");
    }

    #[test]
    fn someone_elses_world() {
        assert!(allowed(true, false, true), "a LAN host with the mod allowed it");
        assert!(!allowed(true, false, false), "a host without the mod, or one that said no");
    }
}

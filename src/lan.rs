//! The consent handshake: a LAN host allowing friends to use lock-on.
//!
//! Lock-on counts as aim assistance, so in someone else's world it only runs when that
//! world's owner explicitly allowed it. When you open your world to LAN, your game runs the
//! world's server, so you are the owner:
//!
//! 1. **A friend asks once** on joining: "is lock-on allowed here?" Without an answer it
//!    asks again after 1, 2, 4 and 8 seconds, then never again (a host without the mod
//!    gets five tiny messages in all).
//! 2. **The host answers** and remembers who asked. Its answer:
//!    - a world hosted from a game window (LAN): the **Allow LAN Friends** setting (off by
//!      default; true is affirmative permission);
//!    - a dedicated server: `allow_lock_on` in its server config file (off by default;
//!      true is affirmative permission; `server_config.rs`).
//! 3. **When the host changes its answer**, it sends the new answer once to everyone who
//!    asked. A heartbeat also refreshes affirmative answers, and stale affirmative updates
//!    expire instead of granting lasting permission.
//! 4. **"No" means leave.** A friend whose host answers "not allowed" (or later turns it
//!    off) is disconnected with a message saying why: that host doesn't want the mod in
//!    their world. An answer ends the asking either way; "no" is never asked again.
//! 5. **No answer means off.** A host without the mod, a vanilla server or any public
//!    server never answers. It can't be told apart from one without the mod, so the player
//!    stays connected and lock-on simply stays off.
//!
//! Both messages are optional payloads, so nobody is refused for not having the mod, and
//! the mod stays `side = "client"`, so it never blocks joining any server.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use mod_api::{ClientHost, Dist, PayloadCodec, PayloadId, PlayerRef, Registrar, ServerHost, ToClient, ToServer, VarInt};

use crate::config;
use crate::server_config;

/// This host's answer: a dedicated server's own switch, or a game window's LAN setting.
fn answer_here(server: &dyn ServerHost) -> bool {
    if server.dist() == Dist::DedicatedServer || server_config::is_dedicated() {
        server_config::allow_lock_on()
    } else {
        config::get().lan_friends
    }
}

/// A friend's question: "is lock-on allowed in this world?"
#[derive(PayloadCodec)]
pub struct AskLockOn {
    /// The handshake's version, for later changes.
    pub protocol: VarInt,
}

/// The host's answer, sent as a reply and again whenever the host changes its mind.
#[derive(PayloadCodec)]
pub struct LockOnAnswer {
    pub protocol: VarInt,
    pub allowed: bool,
}

const ASK_CHANNEL: &str = "enemy_target_lock:ask_lock_on";
const ANSWER_CHANNEL: &str = "enemy_target_lock:lock_on_answer";
/// What a friend sees when a host says no.
const REFUSED: &str = "This world's host doesn't allow Enemy Target Lock. Ask them to turn on \
                       Allow LAN Friends in its settings, or join without the mod.";

/// The payloads' version (bump it if `AskLockOn` or `LockOnAnswer` change shape).
const VERSION: &str = "2";
const PROTOCOL: i32 = 2;
/// How many times a friend asks before giving up, and the first gap (ticks, 20 per second),
/// doubled after each try: asks at 0, 1, 2, 4 and 8 seconds after joining.
const MAX_ASKS: u32 = 5;
const FIRST_RETRY_TICKS: u32 = 20;

static ASK: OnceLock<ToServer<AskLockOn>> = OnceLock::new();
static ANSWER: OnceLock<ToClient<LockOnAnswer>> = OnceLock::new();

// Friend side: what the host said, and the asking schedule.
static HOST_ALLOWS: AtomicBool = AtomicBool::new(false);
static HOST_ANSWERED: AtomicBool = AtomicBool::new(false);
static HOST_REVOKED: AtomicBool = AtomicBool::new(false);
static ANSWER_TICKS_REMAINING: AtomicU32 = AtomicU32::new(0);
static ASKS_SENT: AtomicU32 = AtomicU32::new(0);
static TICKS_TO_ASK: AtomicU32 = AtomicU32::new(0);

const ANSWER_TTL_TICKS: u32 = 600;
const ANSWER_HEARTBEAT_TICKS: u32 = 300;
static HEARTBEAT_TICKS_REMAINING: AtomicU32 = AtomicU32::new(ANSWER_HEARTBEAT_TICKS);

// Host side: who asked, and the answer they last got.
struct HostState {
    asked: Vec<PlayerRef>,
    last_broadcast: bool,
}
static HOST: Mutex<HostState> = Mutex::new(HostState { asked: Vec::new(), last_broadcast: false });

/// Declare both messages (from `Hooks::register`).
pub fn register(registrar: &mut dyn Registrar) {
    let mut payloads = registrar.payload_handlers();
    let mut view = payloads.registrar(VERSION).optional();

    // Host side: answer, and remember who asked so a later change reaches them.
    let ask = view.play_to_server_typed::<AskLockOn>(PayloadId(ASK_CHANNEL.into()), |question, context| {
        let Ok(server) = context.server_host() else {
            return Ok(());
        };
        if question.protocol.0 != PROTOCOL {
            return Ok(());
        }
        let allowed = answer_here(server);
        if let Ok(player) = context.player() {
            let mut host = host_state();
            if !host.asked.contains(&player) {
                host.asked.push(player);
            }
        }
        if let Some(answer) = ANSWER.get() {
            let _ = answer.reply(context, &LockOnAnswer { protocol: VarInt(PROTOCOL), allowed });
        }
        Ok(())
    });
    // Friend side: remember the host's answer (it stops the asking); on "no", leave.
    let answer = view.play_to_client_typed::<LockOnAnswer>(PayloadId(ANSWER_CHANNEL.into()), |answer, context| {
        if answer.protocol.0 != PROTOCOL {
            return Ok(());
        }
        HOST_ANSWERED.store(true, Ordering::Relaxed);
        HOST_REVOKED.store(!answer.allowed, Ordering::Relaxed);
        HOST_ALLOWS.store(answer.allowed, Ordering::Relaxed);
        ANSWER_TICKS_REMAINING.store(if answer.allowed { ANSWER_TTL_TICKS } else { 0 }, Ordering::Relaxed);
        if !answer.allowed
            && let Ok(client) = context.client_host()
        {
            let _ = client.disconnect(Some(mod_api::Text::literal(REFUSED)));
        }
        Ok(())
    });
    let _ = ASK.set(ask);
    let _ = ANSWER.set(answer);
}

/// Friend side, each tick in someone else's world: ask until the host answers, with growing
/// gaps, at most `MAX_ASKS` times. Costs nothing once answered or given up.
pub fn ask_until_answered(host: &mut dyn ClientHost) {
    if HOST_ANSWERED.load(Ordering::Relaxed) {
        return;
    }
    let sent = ASKS_SENT.load(Ordering::Relaxed);
    if sent >= MAX_ASKS {
        return;
    }
    let wait = TICKS_TO_ASK.load(Ordering::Relaxed);
    if wait > 0 {
        TICKS_TO_ASK.store(wait - 1, Ordering::Relaxed);
        return;
    }
    if let Some(ask) = ASK.get() {
        // A host without the mod doesn't take it (`UnsupportedPeer`): no answer, so no lock-on.
        let _ = ask.send(host, &AskLockOn { protocol: VarInt(PROTOCOL) });
    }
    ASKS_SENT.store(sent + 1, Ordering::Relaxed);
    TICKS_TO_ASK.store(FIRST_RETRY_TICKS << sent, Ordering::Relaxed);
}

/// Host side, each server tick: if **Allow LAN Friends** changed since the last answer,
/// send the new one once to everyone who asked. Only a comparison otherwise.
pub fn tell_friends_if_changed(server: &mut dyn ServerHost) {
    let allowed = answer_here(server);
    let mut host = host_state();
    let changed = host.last_broadcast != allowed;
    host.last_broadcast = allowed;
    let Some(answer) = ANSWER.get() else {
        return;
    };
    let remaining = HEARTBEAT_TICKS_REMAINING.load(Ordering::Relaxed);
    let heartbeat = remaining == 0;
    if heartbeat {
        HEARTBEAT_TICKS_REMAINING.store(ANSWER_HEARTBEAT_TICKS, Ordering::Relaxed);
    } else {
        HEARTBEAT_TICKS_REMAINING.store(remaining - 1, Ordering::Relaxed);
    }
    if changed || heartbeat {
        // Failed sends identify players who left; drop their permission state.
        host.asked.retain(|&player| answer.send(server, player, &LockOnAnswer { protocol: VarInt(PROTOCOL), allowed }).is_ok());
    }
}

/// Whether the host of the world this client is in said lock-on is allowed.
pub fn host_allows() -> bool {
    if HOST_REVOKED.load(Ordering::Relaxed) {
        HOST_ALLOWS.store(false, Ordering::Relaxed);
        return false;
    }
    expire_permission();
    HOST_ALLOWS.load(Ordering::Relaxed)
}

pub fn revoked() -> bool {
    HOST_REVOKED.load(Ordering::Relaxed)
}

fn expire_permission() {
    if !HOST_ANSWERED.load(Ordering::Relaxed) {
        HOST_ALLOWS.store(false, Ordering::Relaxed);
        ANSWER_TICKS_REMAINING.store(0, Ordering::Relaxed);
        return;
    }
    let remaining = ANSWER_TICKS_REMAINING.load(Ordering::Relaxed);
    if remaining == 0 {
        HOST_ALLOWS.store(false, Ordering::Relaxed);
    } else {
        ANSWER_TICKS_REMAINING.store(remaining - 1, Ordering::Relaxed);
    }
}

/// Friend side: forget the last answer (on leaving a world), so the next host is asked anew.
pub fn forget() {
    HOST_ALLOWS.store(false, Ordering::Relaxed);
    HOST_ANSWERED.store(false, Ordering::Relaxed);
    HOST_REVOKED.store(false, Ordering::Relaxed);
    ANSWER_TICKS_REMAINING.store(0, Ordering::Relaxed);
    ASKS_SENT.store(0, Ordering::Relaxed);
    TICKS_TO_ASK.store(0, Ordering::Relaxed);
}

fn host_state() -> std::sync::MutexGuard<'static, HostState> {
    HOST.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

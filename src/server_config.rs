//! The dedicated-server switch: `config/enemy_target_lock-server.toml`.
//!
//! On a dedicated server with this mod installed, players with the mod get lock-on while
//! `allow_lock_on` is `false`, **the default**: installing the mod on the server does not
//! grant permission by itself. (A world hosted from a game window uses the
//! in-game **Allow LAN Friends** setting instead; see `lan.rs`.)
//!
//! Only a dedicated server has this file, and only a game window has Allow LAN Friends:
//! `lib.rs` sets up each one's own (see [`set_dedicated`]).
//!
//! Horizon writes [`DEFAULT_TEXT`] when the file is missing, reads it when each world starts
//! (from the world's `serverconfig/` when there is one) and again when it's saved.

use std::sync::atomic::{AtomicBool, Ordering};

/// The file's name inside `config/` (Horizon's default for a server config).
pub const FILE_NAME: &str = "enemy_target_lock-server.toml";

pub const DEFAULT_TEXT: &str = "\
# Enemy Target Lock: dedicated server settings.
#
# Players who also have the mod can use lock-on on this server while this is true.
# Lock-on and aim assist count as aim assistance: set it to false to keep them off here,
# for example on a public server. Players without the mod are never affected.
allow_lock_on = false
";

static ALLOW: AtomicBool = AtomicBool::new(false);
static DEDICATED: AtomicBool = AtomicBool::new(false);

/// Record whether this process is a dedicated server (from `Hooks::register`, which runs first).
pub fn set_dedicated(dedicated: bool) {
    DEDICATED.store(dedicated, Ordering::Relaxed);
}

/// Whether this process is a dedicated server (as opposed to a game window, which may host LAN).
pub fn is_dedicated() -> bool {
    DEDICATED.load(Ordering::Relaxed)
}

/// Whether this dedicated server allows lock-on.
pub fn allow_lock_on() -> bool {
    ALLOW.load(Ordering::Relaxed)
}

/// Horizon read the file: use it. Returns a warning for each line that was ignored.
pub fn file_read(text: &str) -> Vec<String> {
    let (allow, warnings) = parse(text);
    ALLOW.store(allow, Ordering::Relaxed);
    warnings
}

/// `allow_lock_on` from the file's text (`false`, the default, when missing; a value other
/// than `true`/`false` keeps it off, since the owner clearly meant to change something).
pub fn parse(text: &str) -> (bool, Vec<String>) {
    let mut allow = false;
    let mut warnings = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.split_once('#').map_or(raw, |(before, _)| before).trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        match line.split_once('=').map(|(key, value)| (key.trim(), value.trim())) {
            Some(("allow_lock_on", "true")) => allow = true,
            Some(("allow_lock_on", "false")) => allow = false,
            Some(("allow_lock_on", value)) => {
                allow = false;
                warnings.push(format!("line {}: allow_lock_on: `{value}` is not true or false, using false", index + 1));
            }
            _ => warnings.push(format!("line {}: unknown setting `{line}`, ignored", index + 1)),
        }
    }
    (allow, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_by_default() {
        assert_eq!(parse(DEFAULT_TEXT), (false, vec![]));
        assert_eq!(parse(""), (false, vec![]));
    }

    #[test]
    fn the_owner_turns_it_off() {
        assert_eq!(parse("allow_lock_on = false  # public server\n"), (false, vec![]));
    }

    #[test]
    fn mistakes_keep_it_off() {
        let (allow, warnings) = parse("allow_lock_on = yes\nnope = 1\n");
        assert!(!allow);
        assert_eq!(warnings.len(), 2);
    }
}

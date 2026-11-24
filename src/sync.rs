//! Keeps the config file and the in-game settings page saying the same thing.
//!
//! - **File → game.** When Horizon reads the file (at startup, and within a second of it
//!   being saved), its values are used right away, then copied into the game's settings
//!   store on the next tick so the in-game page shows them. (Startup config loading may
//!   read settings but not write them, so the copy waits for the game to be running.)
//! - **Game → file.** When a setting is changed on the in-game page, its values are used
//!   right away and the file is rewritten with them.
//!
//! Writing the file makes Horizon read it again; that read finds nothing new, so the two
//! never bounce back and forth. If both are changed at once, the most recent change wins.

use std::sync::atomic::{AtomicBool, Ordering};

use mod_api::{ClientHost, ConfigPath};

use crate::config::{self, Config, SETTINGS};
use crate::config_file;

/// This mod's id, as in `horizon-mod.toml`.
pub const MOD_ID: &str = "enemy_target_lock";

#[derive(Default)]
pub struct SettingsSync {
    /// The file was read and its values still need copying into the game's settings store.
    copy_to_game_pending: AtomicBool,
}

impl SettingsSync {
    /// Horizon read the config file (`config_loading` / `config_reloading`).
    pub fn file_read(&self, text: &str) {
        let (from_file, warnings) = config_file::parse(text);
        for warning in &warnings {
            eprintln!("{MOD_ID}: {} {warning}", config_file::FILE_NAME);
        }
        config::set(from_file);
        self.copy_to_game_pending.store(true, Ordering::Release);
    }

    /// Each tick: finish copying the file's values into the game's settings store, if
    /// the file was read since. Only settings that differ are written.
    pub fn copy_to_game_if_pending(&self, host: &dyn ClientHost) {
        if !self.copy_to_game_pending.swap(false, Ordering::AcqRel) {
            return;
        }
        let wanted = config::get();
        let stored = stored_in_game(host);
        for setting in SETTINGS {
            let text = setting.value_text(&wanted);
            if setting.value_text(&stored) == text {
                continue;
            }
            host.set_setting(MOD_ID, setting.key, &text);
            if setting.value_text(&stored_in_game(host)) != text {
                eprintln!("{MOD_ID}: the game didn't take `{} = {text}` from {}", setting.key, config_file::FILE_NAME);
            }
        }
    }

    /// A setting was changed on the in-game page (`setting_changed`): use the page's
    /// values and rewrite the file to match.
    pub fn game_changed(&self, host: &dyn ClientHost) {
        let from_game = stored_in_game(host);
        if from_game == config::get() {
            return;
        }
        config::set(from_game);
        let written = ConfigPath::new(config_file::FILE_NAME)
            .map_err(|error| format!("{error:?}"))
            .and_then(|path| {
                host.config_files()
                    .write(&path, config_file::to_text(&from_game).as_bytes())
                    .map_err(|error| format!("{error:?}"))
            });
        if let Err(error) = written {
            eprintln!("{MOD_ID}: couldn't save {}: {error}", config_file::FILE_NAME);
        }
    }
}

/// The values the in-game settings page holds (the player's, or each default).
fn stored_in_game(host: &dyn ClientHost) -> Config {
    config::from_values(|key| host.setting_bool(key), |key| host.setting_f64(key), |key| host.setting_color(key))
}

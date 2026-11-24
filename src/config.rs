//! The mod's settings: what they are, their defaults, and the values in use.
//!
//! They can be changed two ways, kept in step by `sync.rs`:
//! - in-game, on Options > Mods > Enemy Target Lock > Config (registered in `lib.rs`:
//!   `register`; labels and tooltips in `assets/enemy_target_lock/lang/en_us.json`);
//! - in the file `config/enemy_target_lock-client.toml` (`config_file.rs`).
//!
//! Every module reads the values in use with [`get`].

use std::sync::RwLock;

/// Every setting, with the defaults in [`Config::DEFAULT`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    // Targeting
    /// Only targets within this distance (blocks) can be locked or switched to.
    pub lock_range: f64,
    /// A new lock only picks targets within this many degrees of the view direction.
    pub pick_cone_degrees: f32,
    /// Whether other players can be locked onto.
    pub include_players: bool,

    // Multiplayer
    /// When your world is open to LAN, let friends with the mod use lock-on (and keep it on
    /// for you while they're there). Off by default: the host must explicitly enable it.
    pub lan_friends: bool,

    // Lock
    /// The lock is released when the target gets farther away than this (blocks).
    pub max_lock_distance: f64,
    /// The lock is released after the target has been out of sight this long.
    pub max_hidden_seconds: f32,
    /// When the target dies, lock onto the nearest other one (else release the lock).
    pub switch_on_death: bool,

    // Aim
    /// How far up the target's bounding box to aim (0 = feet, 1 = top).
    pub aim_height: f64,
    /// How quickly the player's real aim turns to the target (per second).
    pub aim_speed: f32,
    /// The fastest the aim turns, in degrees per second.
    pub max_turn_degrees_per_second: f32,
    /// Aim bows, tridents and thrown items for drop and target movement.
    pub projectile_aim: bool,

    // First person
    /// How quickly the first-person view catches up with the real aim (per second).
    pub view_speed: f32,

    // Third person
    /// Use the cinematic over-the-shoulder camera when locked in third person.
    pub cinematic_camera: bool,
    pub camera_distance: f64,
    /// Positive = right shoulder, negative = left shoulder.
    pub camera_shoulder: f64,
    pub camera_height: f64,
    /// Where the camera looks between the player (0) and the target (1).
    pub camera_focus: f64,
    /// Field of view while the cinematic camera is in, as a share of the normal FOV.
    pub fov_scale: f32,
    pub camera_position_speed: f32,
    pub camera_angle_speed: f32,

    // HUD
    /// Draw the dot on the locked target.
    pub lock_dot: bool,
    /// The dot's radius in GUI pixels.
    pub dot_size: f32,
    /// How see-through the dot is (0 invisible, 1 solid).
    pub dot_opacity: f32,
    /// The dot's colour, `0xRRGGBB`.
    pub dot_color: u32,
    /// Width of the dark ring around the dot, in GUI pixels (0 = none).
    pub dot_outline: f32,
    /// Hide the normal crosshair while locked.
    pub hide_crosshair: bool,

    // Assist
    /// The subtle aim assist when not locked.
    pub assist_enabled: bool,
    pub assist_max_angle_degrees: f32,
    pub assist_step_degrees: f32,
}

impl Config {
    pub const DEFAULT: Self = Self {
        lock_range: 24.0,
        pick_cone_degrees: 60.0,
        include_players: true,

        lan_friends: false,

        max_lock_distance: 32.0,
        max_hidden_seconds: 3.0,
        switch_on_death: true,

        aim_height: 0.65,
        aim_speed: 20.0,
        max_turn_degrees_per_second: 540.0,
        projectile_aim: true,

        view_speed: 8.0,

        cinematic_camera: true,
        camera_distance: 3.6,
        camera_shoulder: 0.9,
        camera_height: 0.35,
        camera_focus: 0.55,
        fov_scale: 0.85,
        camera_position_speed: 5.0,
        camera_angle_speed: 6.0,

        lock_dot: true,
        dot_size: 0.75,
        dot_opacity: 0.75,
        dot_color: 0xFFFFFF,
        dot_outline: 0.5,
        hide_crosshair: true,

        assist_enabled: true,
        assist_max_angle_degrees: 8.0,
        assist_step_degrees: 0.25,
    };

    /// `max_hidden_seconds` in game ticks (20 per second).
    pub fn max_hidden_ticks(&self) -> u32 {
        (self.max_hidden_seconds * 20.0).round() as u32
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One setting on the in-game page: its key, what kind of control it is, and how it
/// maps onto a [`Config`] field. The page lists them in this order.
pub struct Setting {
    /// The `[section]` it's under in the config file.
    pub section: &'static str,
    /// The setting's key, in the game and in the file. Its label and tooltip are
    /// `enemy_target_lock.setting.<key>` and `enemy_target_lock.setting.<key>.tooltip` in
    /// the language file.
    pub key: &'static str,
    /// What it does: the config file's comment. Kept equal to the in-game tooltip (tested).
    pub help: &'static str,
    pub kind: Kind,
}

pub enum Kind {
    /// On or off.
    Toggle { get: fn(&Config) -> bool, set: fn(&mut Config, bool) },
    /// A slider from `min` to `max` in steps of `step`.
    Range { min: f64, max: f64, step: f64, get: fn(&Config) -> f64, set: fn(&mut Config, f64) },
    /// A colour, `0xRRGGBB` (a colour picker in-game, `#RRGGBB` in the file).
    Color { get: fn(&Config) -> u32, set: fn(&mut Config, u32) },
}

macro_rules! toggle {
    ($section:literal, $key:literal, $field:ident, $help:literal) => {
        Setting {
            section: $section,
            key: $key,
            help: $help,
            kind: Kind::Toggle { get: |c| c.$field, set: |c, v| c.$field = v },
        }
    };
}

macro_rules! color {
    ($section:literal, $key:literal, $field:ident, $help:literal) => {
        Setting {
            section: $section,
            key: $key,
            help: $help,
            kind: Kind::Color { get: |c| c.$field, set: |c, v| c.$field = v },
        }
    };
}

macro_rules! range {
    ($section:literal, $key:literal, $field:ident, $min:expr, $max:expr, $step:expr, $help:literal) => {
        Setting {
            section: $section,
            key: $key,
            help: $help,
            kind: Kind::Range {
                min: $min,
                max: $max,
                step: $step,
                get: |c| c.$field as f64,
                set: |c, v| c.$field = v as _,
            },
        }
    };
}

/// Every setting on the page, in order.
pub const SETTINGS: &[Setting] = &[
    toggle!("targeting", "include_players", include_players, "Whether other players can be locked onto, not just mobs."),
    range!("targeting", "lock_range", lock_range, 4.0, 64.0, 1.0, "Only targets within this many blocks can be locked onto or switched to."),
    range!("targeting", "pick_cone_degrees", pick_cone_degrees, 5.0, 180.0, 5.0, "A new lock only picks targets within this many degrees of where you look."),
    toggle!("multiplayer", "lan_friends", lan_friends, "Let friends who join your world over LAN, and also have this mod, use lock-on there. Turn it off to keep it to yourself: friends with the mod are then disconnected with the reason."),
    range!("lock", "max_lock_distance", max_lock_distance, 4.0, 128.0, 1.0, "The lock is released when the target gets farther than this many blocks away."),
    range!("lock", "max_hidden_seconds", max_hidden_seconds, 0.0, 30.0, 0.5, "The lock is released after the target has been behind blocks this long."),
    toggle!("lock", "switch_on_death", switch_on_death, "When the target dies, lock onto the nearest other one instead of releasing."),
    range!("aim", "aim_height", aim_height, 0.0, 1.0, 0.05, "Where on the target to aim: 0 is its feet, 1 the top of its head."),
    range!("aim", "aim_speed", aim_speed, 1.0, 100.0, 1.0, "How quickly your real aim (where hits and shots go) turns to the target."),
    range!("aim", "max_turn_degrees_per_second", max_turn_degrees_per_second, 30.0, 3600.0, 30.0, "The fastest your aim turns, so big swings stay readable."),
    toggle!("aim", "projectile_aim", projectile_aim, "Aim bows, crossbows, tridents and thrown items for drop and target movement."),
    range!("first_person", "view_speed", view_speed, 1.0, 100.0, 0.5, "How quickly the first-person view catches up with your aim. Lower is smoother."),
    toggle!("third_person", "cinematic_camera", cinematic_camera, "Use the over-the-shoulder camera when locked in third person (F5)."),
    range!("third_person", "camera_distance", camera_distance, 0.5, 16.0, 0.1, "How far behind you the third-person camera sits, in blocks."),
    range!("third_person", "camera_shoulder", camera_shoulder, -4.0, 4.0, 0.1, "How far to the side: positive is the right shoulder, negative the left."),
    range!("third_person", "camera_height", camera_height, -2.0, 4.0, 0.05, "How far above your eyes the third-person camera sits."),
    range!("third_person", "camera_focus", camera_focus, 0.0, 1.0, 0.05, "Where the camera looks: 0 is you, 1 is the target."),
    range!("third_person", "fov_scale", fov_scale, 0.3, 1.5, 0.05, "Field of view while locked in third person, as a share of your normal FOV."),
    range!("third_person", "camera_position_speed", camera_position_speed, 0.5, 50.0, 0.5, "How quickly the third-person camera moves into place. Lower is floatier."),
    range!("third_person", "camera_angle_speed", camera_angle_speed, 0.5, 50.0, 0.5, "How quickly the third-person camera turns. Lower is floatier."),
    toggle!("hud", "lock_dot", lock_dot, "Draw a dot on the locked target."),
    range!("hud", "dot_size", dot_size, 0.5, 12.0, 0.25, "How big the lock dot is: its radius in GUI pixels."),
    range!("hud", "dot_opacity", dot_opacity, 0.0, 1.0, 0.05, "How see-through the lock dot is: 0 is invisible, 1 is solid."),
    color!("hud", "dot_color", dot_color, "The lock dot's colour."),
    range!("hud", "dot_outline", dot_outline, 0.0, 4.0, 0.25, "Width of the dark ring around the lock dot, so it shows on bright backgrounds. 0 turns it off."),
    toggle!("hud", "hide_crosshair", hide_crosshair, "Hide the normal crosshair while a target is locked."),
    toggle!("assist", "assist_enabled", assist_enabled, "When not locked, gently nudge your aim toward the mob under the crosshair."),
    range!("assist", "assist_max_angle_degrees", assist_max_angle_degrees, 0.0, 45.0, 0.5, "Only nudge when the target is within this many degrees of the crosshair."),
    range!("assist", "assist_step_degrees", assist_step_degrees, 0.0, 5.0, 0.05, "The most the assist nudges your aim per tick, in degrees."),
];

impl Setting {
    /// This setting's value in `config` as text: `true`/`false`, or a number rounded to
    /// its slider's precision (`30`, `0.65`). Used for the config file and the game's store.
    pub fn value_text(&self, config: &Config) -> String {
        match self.kind {
            Kind::Toggle { get, .. } => get(config).to_string(),
            Kind::Color { get, .. } => format!("#{:06X}", get(config) & 0xFF_FFFF),
            Kind::Range { get, .. } => {
                // f32 fields widen to values like 0.8500000238; round that noise away.
                let rounded = (get(config) * 10_000.0).round() / 10_000.0;
                format!("{}", if rounded == 0.0 { 0.0 } else { rounded })
            }
        }
    }

    /// Set this setting in `config` from text. Errors (and leaves `config` alone) for a
    /// value that isn't one of this setting's, or a number outside its slider.
    pub fn apply_text(&self, config: &mut Config, text: &str) -> Result<(), String> {
        match self.kind {
            Kind::Toggle { set, .. } => match text {
                "true" => set(config, true),
                "false" => set(config, false),
                _ => return Err(format!("`{text}` is not true or false")),
            },
            Kind::Color { set, .. } => {
                let hex = text.strip_prefix('#').or_else(|| text.strip_prefix("0x")).unwrap_or(text);
                match u32::from_str_radix(hex, 16) {
                    Ok(rgb) if hex.len() == 6 => set(config, rgb),
                    _ => return Err(format!("`{text}` is not a colour like #FF3030")),
                }
            }
            Kind::Range { min, max, set, .. } => {
                let value: f64 = text.parse().ok().filter(|v: &f64| v.is_finite()).ok_or(format!("`{text}` is not a number"))?;
                if !(min..=max).contains(&value) {
                    return Err(format!("{value} is outside {min} to {max}"));
                }
                set(config, value);
            }
        }
        Ok(())
    }
}

/// The setting with key `key`.
pub fn find(key: &str) -> Option<&'static Setting> {
    SETTINGS.iter().find(|setting| setting.key == key)
}

/// The translation key of a setting's label.
pub fn label_key(key: &str) -> String {
    format!("enemy_target_lock.setting.{key}")
}

/// The translation key of a setting's tooltip.
pub fn tooltip_key(key: &str) -> String {
    format!("enemy_target_lock.setting.{key}.tooltip")
}

/// Build a [`Config`] from the game's stored values. `read_toggle`, `read_range` and
/// `read_color` return the player's value for a key (or `None`, which keeps the default).
pub fn from_values(
    read_toggle: impl Fn(&str) -> Option<bool>,
    read_range: impl Fn(&str) -> Option<f64>,
    read_color: impl Fn(&str) -> Option<u32>,
) -> Config {
    let mut config = Config::DEFAULT;
    for setting in SETTINGS {
        match setting.kind {
            Kind::Toggle { set, .. } => {
                if let Some(value) = read_toggle(setting.key) {
                    set(&mut config, value);
                }
            }
            Kind::Color { set, .. } => {
                if let Some(value) = read_color(setting.key) {
                    set(&mut config, value & 0xFF_FFFF);
                }
            }
            Kind::Range { min, max, set, .. } => {
                if let Some(value) = read_range(setting.key).filter(|v| v.is_finite()) {
                    set(&mut config, value.clamp(min, max));
                }
            }
        }
    }
    config
}

static CURRENT: RwLock<Config> = RwLock::new(Config::DEFAULT);

/// The settings in use right now.
pub fn get() -> Config {
    *CURRENT.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Replace the settings in use.
pub fn set(config: Config) {
    *CURRENT.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = config;
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANG: &str = include_str!("../assets/enemy_target_lock/lang/en_us.json");

    #[test]
    fn keys_are_unique_and_valid() {
        let mut seen = std::collections::HashSet::new();
        for setting in SETTINGS {
            assert!(seen.insert(setting.key), "duplicate key {}", setting.key);
            assert!(
                setting.key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.".contains(c))
                    && !setting.key.starts_with('.')
                    && setting.key.len() <= 64,
                "invalid key {}",
                setting.key
            );
        }
    }

    #[test]
    fn defaults_fit_their_sliders() {
        for setting in SETTINGS {
            if let Kind::Range { min, max, step, get, .. } = setting.kind {
                let default = get(&Config::DEFAULT);
                assert!(min < max && step > 0.0 && step <= max - min, "{}", setting.key);
                assert!((min..=max).contains(&default), "{} default {default}", setting.key);
            }
        }
    }

    #[test]
    fn every_setting_has_a_label_and_tooltip() {
        for setting in SETTINGS {
            for key in [label_key(setting.key), tooltip_key(setting.key)] {
                assert!(LANG.contains(&format!("\"{key}\"")), "missing translation {key}");
            }
        }
    }

    #[test]
    fn every_config_field_has_a_setting() {
        // Changing each setting must change the config, so no field is left out of the page.
        assert_eq!(SETTINGS.len(), 29);
        let changed = from_values(|_| Some(false), |_| Some(f64::MAX), |_| Some(0x123456));
        assert_ne!(changed, Config::DEFAULT);
    }

    #[test]
    fn stored_values_are_applied_and_clamped() {
        let config = from_values(
            |key| (key == "include_players").then_some(false),
            |key| match key {
                "lock_range" => Some(30.0),
                "fov_scale" => Some(9.0),
                _ => None,
            },
            |key| (key == "dot_color").then_some(0xFF3030),
        );
        assert_eq!(config.dot_color, 0xFF3030);
        assert!(!config.include_players);
        assert_eq!(config.lock_range, 30.0);
        assert_eq!(config.fov_scale, 1.5);
        assert_eq!(config.aim_speed, Config::DEFAULT.aim_speed);
    }

    #[test]
    fn help_text_matches_the_in_game_tooltip() {
        for setting in SETTINGS {
            let line = format!("\"{}\": \"{}\"", tooltip_key(setting.key), setting.help);
            assert!(LANG.contains(&line), "tooltip and help differ for {}", setting.key);
        }
    }

    #[test]
    fn values_round_trip_through_text() {
        for setting in SETTINGS {
            let text = setting.value_text(&Config::DEFAULT);
            let mut config = Config::DEFAULT;
            setting.apply_text(&mut config, &text).unwrap();
            assert_eq!(config, Config::DEFAULT, "{} via `{text}`", setting.key);
        }
        assert_eq!(find("fov_scale").unwrap().value_text(&Config::DEFAULT), "0.85");
        assert_eq!(find("lock_range").unwrap().value_text(&Config::DEFAULT), "24");
        assert_eq!(find("dot_color").unwrap().value_text(&Config::DEFAULT), "#FFFFFF");
        let mut config = Config::DEFAULT;
        find("dot_color").unwrap().apply_text(&mut config, "#ff3030").unwrap();
        assert_eq!(config.dot_color, 0xFF3030);
    }

    #[test]
    fn bad_text_is_refused() {
        let mut config = Config::DEFAULT;
        assert!(find("lock_range").unwrap().apply_text(&mut config, "far").is_err());
        assert!(find("lock_range").unwrap().apply_text(&mut config, "500").is_err());
        assert!(find("lock_dot").unwrap().apply_text(&mut config, "yes").is_err());
        assert!(find("dot_color").unwrap().apply_text(&mut config, "red").is_err());
        assert!(find("dot_color").unwrap().apply_text(&mut config, "#FFF").is_err());
        assert_eq!(config, Config::DEFAULT);
    }

    #[test]
    fn hidden_seconds_become_ticks() {
        assert_eq!(Config::DEFAULT.max_hidden_ticks(), 60);
    }
}

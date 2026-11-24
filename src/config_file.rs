//! The config file, `config/enemy_target_lock-client.toml`: writing the settings as
//! commented text, and reading them back.
//!
//! The format is a small subset of TOML: `[section]` headers, `key = value` lines with
//! numbers, `true`/`false` or quoted colours (`"#FF3030"`), and `#` comments. Keys are the same as the in-game
//! settings. A missing key keeps its default; an unknown key, a bad value or a number
//! outside its slider's range keeps the default too and is reported as a warning.

use crate::config::{self, Config, Kind, SETTINGS};

/// The file's name inside the game's `config/` folder (Horizon's default for a
/// client config: `<mod id>-client.toml`).
pub const FILE_NAME: &str = "enemy_target_lock-client.toml";

const HEADER: &str = "\
# Enemy Target Lock settings.
#
# These are the same settings as Options > Mods > Enemy Target Lock > Config.
# Edit either one: saving this file while the game runs applies it within about a
# second and updates the in-game page, and changing a setting in-game rewrites this file.
# Delete this file to get the defaults back.
";

/// The whole file for `config`: a header, then each section with every setting's
/// help as a comment, its allowed range, and its value.
pub fn to_text(config: &Config) -> String {
    let mut text = String::from(HEADER);
    let mut section = "";
    for setting in SETTINGS {
        if setting.section != section {
            section = setting.section;
            text.push_str(&format!("\n[{section}]\n"));
        }
        let range = match setting.kind {
            Kind::Toggle { .. } => "true or false".to_owned(),
            Kind::Range { min, max, .. } => format!("{min} to {max}"),
            Kind::Color { .. } => "a colour like #FF3030".to_owned(),
        };
        text.push_str(&format!("# {} ({range})\n", setting.help));
        let value = match setting.kind {
            // Quoted, as TOML writes text: a bare `#` would start a comment.
            Kind::Color { .. } => format!("\"{}\"", setting.value_text(config)),
            _ => setting.value_text(config),
        };
        text.push_str(&format!("{} = {value}\n", setting.key));
    }
    text
}

/// Read a config file's text. Returns the settings and a warning for each line
/// that was ignored.
pub fn parse(text: &str) -> (Config, Vec<String>) {
    let mut config = Config::DEFAULT;
    let mut warnings = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() || (line.starts_with('[') && line.ends_with(']')) {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            warnings.push(format!("line {line_number}: expected `key = value`, got `{line}`"));
            continue;
        };
        let key = key.trim();
        let Some(setting) = config::find(key) else {
            warnings.push(format!("line {line_number}: unknown setting `{key}`, ignored"));
            continue;
        };
        let value = value.trim();
        let value = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(value);
        if let Err(problem) = setting.apply_text(&mut config, value) {
            warnings.push(format!("line {line_number}: {key}: {problem}, using the default"));
        }
    }
    (config, warnings)
}

/// `line` without its `#` comment. A `#` inside quotes (a colour) isn't a comment.
fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (index, character) in line.char_indices() {
        match character {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_file_reads_back_as_the_defaults() {
        let (config, warnings) = parse(&to_text(&Config::DEFAULT));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config, Config::DEFAULT);
    }

    #[test]
    fn changed_values_survive_a_round_trip() {
        let mut changed = Config::DEFAULT;
        changed.lock_range = 30.0;
        changed.include_players = false;
        changed.camera_shoulder = -0.9;
        changed.fov_scale = 0.7;
        changed.dot_color = 0xFF3030;
        let (config, warnings) = parse(&to_text(&changed));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config, changed);
    }

    #[test]
    fn the_file_explains_each_setting() {
        let text = to_text(&Config::DEFAULT);
        assert!(text.contains("[third_person]\n"));
        assert!(text.contains("# Only targets within this many blocks can be locked onto or switched to. (4 to 64)\nlock_range = 24\n"));
        assert!(text.contains("include_players = true"));
        assert!(text.contains("dot_color = \"#FFFFFF\"\n"));
    }

    #[test]
    fn hand_edits_with_comments_and_mistakes() {
        let (config, warnings) = parse(
            "[targeting]\nlock_range = 30   # farther\nlock_dot = yes\npick_cone_degrees = 500\nnope = 1\njunk\ndot_color = \"#00ff00\" # green\n",
        );
        assert_eq!(config.dot_color, 0x00FF00);
        assert_eq!(config.lock_range, 30.0);
        assert_eq!(config.lock_dot, Config::DEFAULT.lock_dot);
        assert_eq!(config.pick_cone_degrees, Config::DEFAULT.pick_cone_degrees);
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        assert!(warnings[0].starts_with("line 3:"), "{warnings:?}");
    }
}

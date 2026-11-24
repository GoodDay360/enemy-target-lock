# Enemy Target Lock — Spec

A client-side native Rust mod for Minecraft, built with the Horizon launcher.

## Goal

Press a button to **lock onto the best mob in front of you**, without having to aim
at it first. While locked, the camera smoothly follows the target until the lock ends,
and you can switch to the next target on either side. When the target dies, the lock
moves on to the nearest mob by itself.

## Current state

### Controls
| Action | Input |
|---|---|
| Lock onto the best target in view / release the lock | **Middle mouse** |
| Switch to the next target right / left (while locked) | **Scroll down / up** (same direction as the hotbar) |
| Attack / use an item on the locked target | **Left / right click**: goes to the target, the camera keeps gliding |
| Cinematic lock camera | Lock while in **third person from behind** (F5 once) |

- Input is ignored while a menu, chat or inventory screen is open.
- A middle click is only cancelled when it locks or unlocks. With nothing to lock, it
  still picks the block as usual.
- While locked, scrolling switches targets instead of changing the hotbar slot.

### Picking a target (`targeting::pick_best`)
1. Find every entity within **24 blocks** (`scan::nearby_candidates`).
2. Keep mobs and players only (`targeting::is_lockable`), not the player themselves.
3. Keep those **in line of sight** (a block raycast from the eye to the target's chest)
   and within **60°** of the view direction.
4. Pick the lowest score: degrees from the view center + 0.5 × distance in blocks.
   The target nearest the middle of the screen wins, and closer ones break ties.

All mobs count, passive ones and mods' mobs included. Objects such as boats, minecarts,
item frames, armor stands, dropped items and projectiles don't. On the client the API
can't tell whether another entity is living, so this works from a list of non-mob type ids.

### Switching targets (`targeting::pick_next`)
- Scroll down picks the nearest visible target to the **right** of the current one;
  scroll up picks the nearest to the **left**. This matches the way the hotbar moves.
- It only looks up to 90° either side, and the order doesn't wrap around.
  With nothing on that side, the lock stays where it is.

### When the target dies (`Mod::handle_target_death`, `deaths.rs`)
- The lock **switches by itself to the nearest other mob**: the closest visible one within
  24 blocks, in any direction (`targeting::pick_nearest`). The camera glides over to it.
- With no other target around, the lock is released.
- Death is noticed as soon as the death animation starts, not when the body disappears.
  The client can't read other mobs' health, but `render_living_pre` reports each drawn
  mob's `death_time`, which is above 0 once it has died. Mobs seen dying are never picked
  as the next target.
- A target that vanishes without a death being seen (killed out of view, despawned)
  is handled the same way.

### While locked (`Mod::update_lock`, `Mod::turn_camera`)
The **player's real aim** and the **drawn camera** move separately:

- **Real aim: fast and precise** (`Mod::turn_camera`, `smooth.rs`). This is the player's
  rotation, which decides where attacks, arrows and thrown items go and where the player
  model faces. Every frame it turns toward the target's chest (65% up its bounding box), or
  toward the ballistic aim point while holding a projectile item.
  - Each tick, the player's eye and the target's aim point are recorded. Each frame blends
    the last two records by the frame's partial tick, so the followed point moves as smoothly
    as the game draws entities.
  - It closes a fixed share of the remaining angle per second (`AIM_RESPONSE_PER_SECOND` = 20).
    It's the same at any frame rate, never overshoots, and turns at most 540°/s.
  - Errors under 0.05° are left alone, so it rests instead of trembling. Pitch is limited to ±90°.
- **Drawn camera: trails behind smoothly** (`camera_pose` hook). Only the picture moves.
  - **First person** (`view_lag.rs`): the view eases after the real aim
    (`VIEW_RESPONSE_PER_SECOND` = 8, about 87% in a quarter second), so a quick correction
    for an arrow's drop or a moving target is felt as a smooth glide, not a jump.
  - **Third person from behind**: the cinematic camera (below).
  - Both fade in when you lock and back out when you release, so the camera ends exactly on
    the player's view again.
- The mouse doesn't turn the camera while locked: the frame's mouse sensitivity is muted
  (`calculate_player_turn`), so the mouse can't fight the lock. The saved option is unchanged.
- A round **dot** is drawn on the target's chest (`hud.rs`): small, white, slightly see-through
  (75%), 0.75 GUI pixels in radius, with a very thin dark outline (0.5) so it shows on bright
  backgrounds. Size, opacity, colour and outline are settings. It's built from thin
  wedges (`shapes.rs`), since the GUI draws only rectangles and quads; the pieces don't
  overlap, so the transparency is even.
- The normal **crosshair is hidden** while locked, so the dot is the only aim marker.
  It comes back as soon as the lock ends.
- **Clicks go to the target** (`attack.rs`): left click attacks it and right click uses the
  held item on it, wherever the crosshair is at that moment. The camera doesn't snap; it keeps
  trailing smoothly. This uses the game's targeting override (`set_targeting`), which is set while
  locked and cleared when the lock ends. The game still checks reach and line of sight, and
  the server's own reach and cooldown checks are unchanged.

### Projectiles (`ballistics.rs`)
Holding a bow, crossbow, trident, snowball, egg, ender pearl, splash/lingering potion,
bottle o' enchanting or wind charge while locked, the view aims where the projectile
will **actually land**, not straight at the target:
- **Drop and drag:** the flight is simulated tick by tick as the game does it (launch
  speed, gravity and air drag per item). The launch angle that puts it on the target's
  chest is found by bisection, searching direct shots up to 45° above level.
- **Moving targets are led:** the target's motion over the last tick predicts where it will
  be when the projectile arrives, refined a few times (`ballistics::solve`).
- **Bow draw:** a bow is always aimed for a **full draw**. Following the draw as it builds
  up made the aim jump up when drawing started (a barely drawn arrow is slow and drops a
  lot) and come back down as it filled. Shots released early will land a little short.
- **Potions and bottles** are thrown 20° above the view by the game, so the view aims
  20° lower to match.
- Only the **real aim** uses the ballistic angle. The drawn view trails it smoothly, so
  raising the bow for a long shot is a glide, not a jump.
- **Out of range:** it falls back to aiming straight at the target.
- The item checked is the one being used, else the main hand's, else the off hand's
  (`scan::held_projectile`).
- Not modelled: a crossbow loaded with a firework (assumed an arrow), the player's own
  movement added to the projectile, random spread, and projectiles slowing in water.

### Third-person cinematic camera (`cinematic.rs`)
Used while locked in **third person from behind**. Locking in first person or in the
front-facing view keeps the normal camera.
- **The player turns to the target quickly** (the real aim, above), so the player model faces
  it. The player's own view isn't what's drawn, so this doesn't feel abrupt.
- **The camera is separate from the player** (`camera_pose` hook). It sits behind the player
  on the side away from the target (3.6 blocks back, 0.9 to the right, 0.35 above the eyes),
  looking at a point 55% of the way from the player to the target, so both stay in frame.
- **It drifts slowly into place.** Its position and direction each ease toward that spot
  every frame (5 and 6 per second), so it trails the action instead of following it rigidly.
- **The FOV narrows to 85%** of normal for a tighter, cinematic picture (`compute_fov` hook).
- **It eases in when you lock and back out when you release**, or when you leave third person.
  The view blends between the normal camera and the cinematic one, so nothing snaps.
- **It doesn't go through walls.** It's pulled in toward the player when a block is in the
  way (`scan::clip_camera`), like the normal third-person camera.
- Only the picture moves. The player's aim, the first-person hand and chunk loading still
  follow the player.

### When the lock ends (`lock.rs`)
- The target died or vanished and no other target is around (see above).
- The target is more than **32 blocks** away.
- The target has been out of sight (behind blocks) for **3 seconds** (60 ticks).
- Middle click again.

### Settings (`config.rs`, `config_file.rs`, `sync.rs`)
Every tunable value can be changed in two places, and they stay in step:
- **In-game:** Options > Mods > Enemy Target Lock > **Config**. Horizon draws the page
  (sliders and toggles, each with a tooltip) and tells the mod when one changes
  (`setting_changed`). The change applies immediately and the config file is rewritten.
- **The config file:** `config/enemy_target_lock-client.toml` in the game folder. Horizon
  writes it with the defaults and comments when it's missing (`register_configs`), and
  reads it at startup and within a second of each save (`config_loading`,
  `config_reloading`). Its values apply immediately and are copied into the in-game page
  on the next tick.

Startup config loading may read settings but not write them, so the copy into the page
waits for the first tick. Writing the file makes Horizon read it again, but that read
finds nothing new, so the two never bounce back and forth. If both change at once, the
most recent change wins. In the file, an unknown key, a bad value or a number outside
its range keeps the default and prints a warning to the game's output.

The defaults are the values described in this spec.

| Group | Settings |
|---|---|
| Integrated-world owner | Always allowed |
| LAN guests | Allow LAN Friends (off by default) |
| Targeting | Target Players, Lock Range, Lock Cone |
| Lock | Release Distance, Release When Hidden, Switch Target on Kill |
| Aim | Aim Height, Aim Speed, Max Turn Speed, Projectile Aim |
| First person | First-Person Camera Speed |
| Third person | Cinematic Third-Person Camera, Camera Distance / Shoulder / Height / Focus, Locked FOV, Camera Move / Turn Speed |
| HUD | Lock Dot, Lock Dot Size / Opacity / Colour / Outline, Hide Crosshair While Locked |
| Assist | Aim Assist, Aim Assist Range, Aim Assist Strength |

- All settings are defined in one table, `config::SETTINGS`: file section, key, help text
  (the file's comment, kept equal to the tooltip), slider range and step, and which
  `Config` field it sets. Labels and tooltips are in
  `assets/enemy_target_lock/lang/en_us.json`, as `enemy_target_lock.setting.<key>` and
  `<key>.tooltip`.
- Modules read the current values with `config::get()`. The defaults are `Config::DEFAULT`.
- Adding a setting: add a field to `Config` and `DEFAULT`, a row to `SETTINGS`, and the two
  lines to `en_us.json`. The unit tests fail if a key is invalid, a default is outside its
  slider, a translation is missing, or the help text and tooltip differ.

### Where it works (`session.rs`, `lan.rs`)
Lock-on, the aim assist, sending clicks to the target and projectile aiming count as aim assistance, so
they only run where the world's owner allowed it:

| Where | Lock-on |
|---|---|
| Your integrated world | on for its owner; **Allow LAN Friends** controls guests and is off by default |
| A friend's LAN world | on only if their mod answers "allowed" |
| A dedicated server with the mod | off unless `allow_lock_on` is `true` in `config/enemy_target_lock-server.toml`; the default is off |
| Any other server, Realms, a host without the mod | off; this client disconnects if the permission channel is unsupported |

- **Consent handshake** (`lan.rs`, protocol 2): a friend's game sends `enemy_target_lock:ask_lock_on` on joining.
  If the peer's negotiated payload table definitively lacks the optional channel (`UnsupportedPeer`), the client
  disconnects immediately. While the protocol is still negotiating or a send is transiently refused, it retries
  after 1, 2, 4 and 8 seconds. The host's game replies on
  `enemy_target_lock:lock_on_answer` with its **Allow LAN Friends** setting (off by default) and remembers who
  asked; when the host changes the setting, its game sends the new answer to them. Periodic heartbeats refresh
  permission so lost updates expire.
- **"No" disconnects:** a friend whose host answers "not allowed", or turns it off later, is disconnected with
  a message saying why.
- **Unsupported channel means leave:** a host without a compatible mod does not negotiate the optional
  permission channel, so this mod disconnects its own user immediately. A peer that supports the channel but
  doesn't answer remains fail-closed; lock-on stays off while bounded retries continue.
- Both payloads are `optional()`, and the mod stays `side = "client"`: the server does not require this mod,
  and players who don't have it are not refused.
- "Your own world" is `client_server().integrated_local`, which identifies the local owner. The owner is always allowed; **Allow LAN Friends** separately controls guests and is off by default.
- Where it's off, middle click does what it normally does, and a chat line explains why, once a session.
- **Dedicated servers** (`server_config.rs`) don't use Allow LAN Friends. They answer the same handshake with
  `allow_lock_on` from their own `config/enemy_target_lock-server.toml` (a Horizon `Server` config, read per
  world, from the world's `serverconfig/` when there is one), **off by default**. The mod tells them apart with `dist()` (recorded at registration).
- **Each kind of host sees only its own switch:** a game window registers the settings page and the client
  config file (with Allow LAN Friends); a dedicated server registers neither, only its server config file. Not yet confirmed: whether Horizon's dedicated-server
  export loads a `side = "client"` mod at all; if it doesn't, such a server never answers and lock-on stays off.

### Subtle aim assist (when not locked)
- It only works while the crosshair is already on a mob or player.
- It turns at most **0.25° per tick** and only within **8°** of the target.

## Code layout

`lib.rs` only wires the Horizon hooks to the modules below. Horizon requires the
`Hooks` impl to be at the crate root.

| File | Contents |
|---|---|
| `src/lib.rs` | Hooks: `mouse_button_pre`, `mouse_scrolling`, `calculate_player_turn`, `client_tick_pre`, `register` (settings, payloads), `setting_changed`, `register_configs`, `config_loading`, `config_reloading`, `render_frame_pre`, `camera_pose`, `compute_fov`, `render_living_pre`, `gui_layer_pre`, `gui_render_post` |
| `src/input.rs` | Middle button code, scroll direction → `Side`, muted mouse sensitivity |
| `src/smooth.rs` | Per-frame position blending (`Track`, target velocity), easing (`ease_toward_at`, `ease_fraction`), turn speeds, `FrameClock` |
| `src/ballistics.rs` | Projectile items, flight simulation, launch-angle solver with target lead (pure math) |
| `src/view_lag.rs` | First-person view trailing the real aim, fading in/out (pure math) |
| `src/cinematic.rs` | Third-person lock camera: where it sits, easing in/out, FOV scale (pure math) |
| `src/attack.rs` | Point clicks at the locked target (`direct_clicks_at`) |
| `src/session.rs` | Where targeting is allowed (integrated-world owner, or a remote host's consent for guests) |
| `src/lan.rs` | The consent handshake: ask the host, answer friends |
| `src/server_config.rs` | A dedicated server's own switch, `allow_lock_on` (off by default) |
| `src/scan.rs` | World lookups: player view, nearby candidates, line of sight, camera wall clipping, camera view checks, held projectile |
| `src/targeting.rs` | `is_lockable`, `pick_best`, `pick_next`, `pick_nearest` (pure logic, no `host`) |
| `src/deaths.rs` | `DeathWatch`: mobs seen playing their death animation since the last tick |
| `src/lock.rs` | `LockState` (target, hidden ticks, position track), unlock limits |
| `src/camera.rs` | Aim point, angles, `step_toward`, `assist_step` |
| `src/hud.rs` | The lock dot, crosshair layer id |
| `src/shapes.rs` | Round HUD shapes (disc, ring) built from quads, colour with opacity (pure math) |
| `src/config.rs` | Settings: `Config` and defaults, the `SETTINGS` table, value text, `get()` |
| `src/config_file.rs` | The commented config file: write (`to_text`) and read (`parse`) |
| `src/sync.rs` | Keeping the config file and the in-game page in step (`SettingsSync`) |
| `assets/enemy_target_lock/lang/en_us.json` | Settings labels and tooltips |

### Horizon API used
| Need | API |
|---|---|
| Middle click | `Hooks::mouse_button_pre` (`button`, `pressed`; return `Flow::Cancel` to swallow) |
| Scroll | `Hooks::mouse_scrolling` (`scroll[1]`) |
| Nearby entities | `host.level(host.dimension())` → `level.entities_in(host, min, max)` |
| Line of sight | `level.clip(host, from, to, ClipBlock::Collider, ClipFluid::None)` |
| Turn the camera | `host.set_local_rotation(yaw, pitch)`, called every frame from `Hooks::render_frame_pre` (`world_partial_tick`). It also moves the previous-frame angle, so the game doesn't blend it; calling it once per tick makes the view jump in steps. |
| Clicks go to the target | `host.targeting()` / `host.set_targeting(ClientTargeting { entity, .. })` |
| Trailing / cinematic camera | `Hooks::camera_pose` (`position`, `yaw`, `pitch`, `partial`), `host.camera_type()` (`FirstPerson`, `ThirdPersonBack`) |
| Held item / bow draw | `host.living(player)` → `living.hand_item(host, Hand)` → `stack.item().id(host)`; `host.local_item_use()` (`hand`) |
| In-game settings | `Hooks::register` → `registrar.register_setting(SettingSpec::toggle / ::range(..).tooltip(..))`; `host.setting_bool(key)`, `host.setting_f64(key)`, `host.set_setting(mod_id, key, text)`; `Hooks::setting_changed` |
| Config file | `Hooks::register_configs` (`event.configs.push(Config::new(mod_id, ConfigKind::Client, default))`), `Hooks::config_loading` / `config_reloading` (`event.config.text`); `host.config_files().write(&ConfigPath::new(name)?, bytes)` |
| Cinematic FOV | `Hooks::compute_fov` (`degrees`) |
| Stop the mouse turning | `Hooks::calculate_player_turn` (`mouse_sensitivity`, `cinematic_camera_enabled`) |
| Target died | `Hooks::render_living_pre` (`entity.id`, `entity.death_time`; return `Flow::Continue`) |
| Hide the crosshair | `Hooks::gui_layer_pre` (`layer == "minecraft:crosshair"`; return `Flow::Cancel`) |
| Dot | `Hooks::gui_render_post` + `host.project(point)` + `DrawContext::quad` |
| Colour setting | `SettingSpec::color`, `host.setting_color(key)` |

## Still to do

### Targeting and camera
- [ ] Better aim point for very tall or wide mobs (Ender Dragon, Ghast)

### Input, HUD, config, release
- [ ] Let players rebind the lock (a registered key mapping) instead of a fixed middle click
- [ ] HUD text: target name and distance
- [ ] Sound or action-bar message when a lock starts or ends
- [ ] Run the in-game test checklist (below)

### Done
- [x] Split the code into modules
- [x] Middle click locks onto the best target in view, with no need to aim at it
- [x] Every mob can be targeted
- [x] Scroll switches targets left and right
- [x] Unlock on gone, too far, or hidden for 3 s
- [x] Dot on the locked target
- [x] Left/right clicks go to the locked target without snapping the camera
- [x] Smooth camera: per-frame easing toward blended positions, mouse muted while locked
- [x] Removed the red glow on the locked target (the dot is the only marker)
- [x] When the target dies, switch to the nearest other target (or release if none)
- [x] `README.md` describes the controls
- [x] Hide the crosshair while locked
- [x] Third-person cinematic lock camera: over the shoulder, eased in and out, narrower FOV, no wall clipping
- [x] Projectiles aim for drop, drag and target movement
- [x] Real aim is fast and precise; the drawn camera trails it smoothly (first and third person)
- [x] In-game settings page with every tunable value (`config.rs`)
- [x] Commented config file kept in step with the in-game page (`config_file.rs`, `sync.rs`)
- [x] Round, see-through lock dot with size, opacity, colour and outline settings
- [x] Unit tests for the pure logic (47 tests, `scripts/unit-tests.sh`)
- [x] CI: build the `.hmod` on a push to `build` and upload it to a draft release

## Testing

- **Unit tests:** the modules that don't use the API (`ballistics`, `camera`, `cinematic`, `config`,
  `config_file`, `input`, `shapes`, `smooth`, `targeting`, `view_lag`) are tested without the game.
  `cargo test` needs the SDK at `.horizon/sdk/`, so run them with `scripts/unit-tests.sh`, which
  compiles them together with `rustc`. Add a new pure module to its list.
- **CI:** `.github/workflows/build.yml` runs only on a push to the `build` branch
  (`git push origin main:build`): the unit tests, `cargo horizon-mod check` and `cargo horizon-mod build`,
  then the `.hmod` uploaded to the draft release `v<version>` (see `docs/PUBLISH.md`).
- **Build:** `horizon-launcher native-build . --instance "<instance>" --trust-native-code`
- **In-game checklist:**
  - [ ] Middle click with several mobs in view locks the one nearest the center
  - [ ] Middle click with nothing in view picks the block as usual
  - [ ] Mobs behind a wall aren't picked; a target that hides for 3 s is released
  - [ ] Scroll switches right/left and doesn't change the hotbar slot while locked
  - [ ] Killing the target switches to the nearest other mob as the death animation starts;
        killing the last one releases the lock
  - [ ] The target despawns or walks past 32 blocks
  - [ ] The player dies or changes dimension
  - [ ] Passive mobs and players can be locked; boats, item frames and armor stands can't
  - [ ] Nothing happens while a menu or chat is open
  - [ ] On a server or a host without the mod: the client disconnects when the peer reports the permission channel unsupported
  - [ ] LAN, default (Allow LAN Friends on): a friend with the mod gets lock-on; turning it off on the host
        disconnects them with the message; with it off, a friend with the mod who joins is disconnected
  - [ ] A friend without the mod joins any LAN world normally
  - [ ] Attacking while the camera is still gliding hits the target; holding attack to mine works when not locked
  - [ ] Drawing a bow or trident doesn't tilt the view up and back
  - [ ] Bow (full draw), crossbow, trident, snowball, potion hit a still target at 5, 15 and 30 blocks
  - [ ] Arrows hit a target walking sideways
  - [ ] First person: the view glides smoothly while drawing a bow and when the target moves; no jumps
  - [ ] The lock dot stays on the target while the view is trailing
  - [ ] The lock dot is round and see-through; changing its size, opacity, colour and outline
        in-game shows immediately
  - [ ] Right click uses the held item on the target (e.g. trades with a locked villager)
  - [ ] Third person: the player faces the target, the camera drifts over the shoulder, the FOV narrows
  - [ ] Third person: locking, releasing and pressing F5 while locked all ease smoothly with no snap
  - [ ] Third person: the camera doesn't pass through walls in tight spaces
  - [ ] The camera glides with no shaking, at 30, 60 and uncapped fps, with both you and the target moving
  - [ ] The mouse doesn't move the view while locked, and works normally after release
  - [ ] Options > Mods > Enemy Target Lock > Config opens, shows every setting with its tooltip,
        and a change takes effect immediately (e.g. turn off Lock Dot while locked)
  - [ ] `config/enemy_target_lock-client.toml` is created with comments; changing a setting
        in-game rewrites it; saving an edit applies within a second and shows on the page
  - [ ] A typo in the file keeps that setting's default and prints a warning
  - [ ] The crosshair disappears while locked and comes back after release (and after the last kill)

## Notes

- **Multiplayer:** many servers ban aim lock and anti-cheat plugins may flag it. Consider a
  singleplayer-only option, or leaving players out by default.

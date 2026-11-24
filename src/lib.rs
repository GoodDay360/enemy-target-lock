//! Enemy target lock: middle-click to lock the camera onto the mob nearest the
//! center of your view, scroll to switch targets, middle-click again to release.
//! While locked, clicks go to the target without moving the camera, and when the
//! target dies the lock moves on to the nearest one. In third person, a cinematic
//! over-the-shoulder camera frames the player and the target.
//! Without a lock, a subtle aim assist nudges the view toward the mob under the crosshair.
//!
//! This file only wires Horizon hooks to the feature modules:
//! - `input`: mouse codes and scroll direction
//! - `scan`: the player's view, nearby mobs, line of sight (world lookups)
//! - `targeting`: which entities count, and which one to pick or switch to
//! - `lock`: the locked target and when the lock ends
//! - `deaths`: which mobs are playing their death animation
//! - `smooth`: per-frame blending and easing so the locked camera glides
//! - `cinematic`: the third-person lock camera and its FOV
//! - `view_lag`: the first-person view trailing the player's real aim
//! - `attack`: send clicks to the locked target
//! - `ballistics`: aiming bows, tridents and thrown items so they land
//! - `camera`: aim angles and turn limiting
//! - `hud`: the dot on the locked target, hiding the crosshair
//! - `shapes`: round HUD shapes built from quads
//! - `config`: the settings, their defaults and the values in use
//! - `config_file`: the commented config file in `config/`
//! - `sync`: keeping the config file and the in-game settings page in step
//! - `session`: where targeting is allowed (integrated worlds with explicit opt-in, or remote worlds whose host allows it)
//! - `lan`: the consent handshake (a LAN host allowing friends to use lock-on)
//! - `server_config`: a dedicated server's own switch (off by default)
use std::sync::Mutex;

use mod_api::{self, ClientHost, CrosshairTarget, EntityRef, Flow, Hooks};

mod attack;
mod ballistics;
mod camera;
mod cinematic;
mod config;
mod config_file;
mod deaths;
mod hud;
mod input;
mod lan;
mod lock;
mod scan;
mod server_config;
mod session;
mod shapes;
mod smooth;
mod sync;
mod targeting;
mod view_lag;

#[derive(Default)]
struct Mod {
    lock: lock::SharedLock,
    frame_clock: Mutex<smooth::FrameClock>,
    deaths: deaths::DeathWatch,
    cinematic: Mutex<cinematic::CinematicCamera>,
    view_lag: Mutex<view_lag::ViewLag>,
    settings_sync: sync::SettingsSync,
    session: session::SessionGate,
    camera_clock: Mutex<smooth::FrameClock>,
}

impl Mod {
    /// Middle click: release the current lock, or lock onto the best target in view.
    /// Returns whether the click was used.
    fn toggle_lock(&self, host: &dyn ClientHost) -> bool {
        if self.lock.is_locked() {
            self.lock.with(|state| state.release());
            return true;
        }
        let Some(view) = scan::player_view(host) else {
            return false;
        };
        let candidates = scan::nearby_candidates(host, &view);
        let Some(target) = targeting::pick_best(view.yaw, view.pitch, &candidates) else {
            return false;
        };
        self.lock.with(|state| {
            state.lock(target);
            Self::record_positions(host, &view, target, state);
        });
        true
    }

    /// Scroll while locked: switch to the next target on that side, if there is one.
    fn switch_target(&self, host: &dyn ClientHost, side: input::Side) {
        let (Some(current), Some(view)) = (self.lock.target(), scan::player_view(host)) else {
            return;
        };
        let Some(current_aim) = scan::aim_at_entity(host, &view, current) else {
            return;
        };
        let candidates: Vec<_> =
            scan::nearby_candidates(host, &view).into_iter().filter(|c| c.target != current).collect();
        if let Some(next) = targeting::pick_next(current_aim.yaw, side, &candidates) {
            self.lock.with(|state| state.switch_to(next));
        }
    }

    /// Each tick while locked: when the target has died (or vanished), move on to the
    /// nearest other mob, or release the lock if there is none.
    /// Returns the target to keep following.
    fn handle_target_death(&self, host: &dyn ClientHost, view: &scan::View, target: EntityRef) -> Option<EntityRef> {
        let dying = self.deaths.take();
        if !dying.contains(&target) && target.bounding_box(host).is_some() {
            return Some(target);
        }
        let candidates: Vec<_> = scan::nearby_candidates(host, view)
            .into_iter()
            .filter(|c| c.target != target && !dying.contains(&c.target))
            .collect();
        let next = if config::get().switch_on_death { targeting::pick_nearest(&candidates) } else { None };
        self.lock.with(|state| match next {
            Some(next) => state.switch_to(next),
            None => state.release(),
        });
        next
    }

    /// Each tick while locked: record positions for smooth turning, or release the
    /// target once it's gone, too far away, or hidden for too long.
    fn update_lock(&self, host: &dyn ClientHost, view: &scan::View, target: EntityRef) {
        let in_range = scan::aim_at_entity(host, view, target).is_some_and(|aim| aim.distance <= config::get().max_lock_distance);
        let visible = in_range && scan::has_line_of_sight(host, view, target);
        self.lock.with(|state| {
            if !in_range || !state.track_visibility(visible) || !Self::record_positions(host, view, target, state) {
                state.release();
            }
        });
    }

    /// Push this tick's eye and aim point into the lock's track. False if the target is gone.
    fn record_positions(host: &dyn ClientHost, view: &scan::View, target: EntityRef, state: &mut lock::LockState) -> bool {
        let Some(bounds) = target.bounding_box(host) else {
            return false;
        };
        state.track.push(view.eye, camera::aim_point(bounds));
        true
    }

    /// Each frame while locked: turn the player's real aim quickly toward the target's
    /// blended position (holding a projectile item: where the shot will actually land).
    /// What's drawn doesn't jump with it: the camera trails it smoothly (`camera_pose`).
    fn turn_camera(&self, host: &mut dyn ClientHost, partial_tick: f32, seconds: f32) {
        let Some(((eye, point), target_velocity)) =
            self.lock.with(|state| Some((state.track.at(partial_tick)?, state.track.target_velocity())))
        else {
            return;
        };
        let Some(view) = scan::player_view(host) else {
            return;
        };
        let projectile = if config::get().projectile_aim { scan::held_projectile(host, &view) } else { None };
        let (yaw, pitch) = match projectile.and_then(|p| ballistics::solve(p, scan::launch_point(eye), point, target_velocity)) {
            Some(shot) => (shot.yaw, shot.pitch),
            None => match camera::aim_at(eye, point) {
                Some(aim) => (aim.yaw, aim.pitch),
                None => return,
            },
        };
        let turned = smooth::ease_toward_at(config::get().aim_speed, view.yaw, view.pitch, yaw, pitch, seconds);
        if let Some([yaw, pitch]) = turned {
            let _ = host.set_local_rotation(yaw, pitch);
        }
    }

    /// Each frame: where the cinematic camera should be (None when it should ease out:
    /// not locked, or not in third person from behind).
    fn cinematic_goal(&self, host: &dyn ClientHost, partial_tick: f32) -> Option<cinematic::Pose> {
        if !self.lock.is_locked() || !config::get().cinematic_camera || !scan::third_person_behind(host) {
            return None;
        }
        let (eye, point) = self.lock.with(|state| state.track.at(partial_tick))?;
        let mut pose = cinematic::desired_pose(eye, point)?;
        pose.position = scan::clip_camera(host, eye, pose.position);
        Some(pose)
    }

    /// Each tick while not locked: nudge toward the mob under the crosshair.
    fn assist(&self, host: &mut dyn ClientHost, view: &scan::View) {
        if !config::get().assist_enabled {
            return;
        }
        let Some(CrosshairTarget::Entity { entity, .. }) = host.crosshair_target() else {
            return;
        };
        if !entity.kind(host).is_some_and(|kind| targeting::is_lockable(kind.id(host).as_str())) {
            return;
        }
        let Some(aim) = scan::aim_at_entity(host, view, entity) else {
            return;
        };
        if let Some([yaw, pitch]) = camera::assist_step(view.yaw, view.pitch, aim.yaw, aim.pitch) {
            let _ = host.set_local_rotation(yaw, pitch);
        }
    }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn client_disconnecting(&self, _host: &dyn mod_api::ClientHost, _event: &mut mod_api::ClientDisconnectingEvent) {
        self.session.revoke();
    }

    /// Put every setting on the mod's page in Options > Mods (its Config button).
    fn register(&self, registrar: &mut dyn mod_api::Registrar, context: &mod_api::RegistrationContext) {
        // A dedicated server has no settings page (and no Allow LAN Friends): it uses its own
        // server config file instead.
        let dedicated = context.dist() == mod_api::Dist::DedicatedServer;
        server_config::set_dedicated(dedicated);
        // The consent handshake (lan.rs), on both kinds of host.
        lan::register(registrar);
        if dedicated {
            return;
        }
        let defaults = config::Config::DEFAULT;
        for setting in config::SETTINGS {
            let label = config::label_key(setting.key);
            let spec = match setting.kind {
                config::Kind::Toggle { get, .. } => mod_api::SettingSpec::toggle(setting.key, label, get(&defaults)),
                config::Kind::Range { min, max, step, get, .. } => {
                    mod_api::SettingSpec::range(setting.key, label, min, max, step, get(&defaults))
                }
                config::Kind::Color { get, .. } => mod_api::SettingSpec::color(setting.key, label, get(&defaults)),
            };
            if let Err(error) = registrar.register_setting(spec.tooltip(config::tooltip_key(setting.key))) {
                eprintln!("enemy_target_lock: setting {} was refused: {error}", setting.key);
            }
        }
    }

    /// The world this game hosts ticked: if the host changed Allow LAN Friends, tell the
    /// friends who asked (once; otherwise just a comparison).
    fn server_tick(&self, host: &mut dyn mod_api::ServerHost, _event: &mut mod_api::ServerTickEvent) {
        lan::tell_friends_if_changed(host);
    }

    /// The player changed a setting on the mod's page: use it and save it to the config file.
    fn setting_changed(&self, host: &mut dyn ClientHost, _event: &mut mod_api::SettingChangedEvent) {
        self.settings_sync.game_changed(host);
    }

    /// The config file, written with the defaults when it's missing.
    fn register_configs(&self, _side: mod_api::HostSide, event: &mut mod_api::RegisterConfigsEvent) {
        let default = config_file::to_text(&config::Config::DEFAULT);
        // Each kind of host gets only its own file: a game window the client settings (with
        // Allow LAN Friends), a dedicated server its server switch.
        if server_config::is_dedicated() {
            event.configs.push(mod_api::Config::new(sync::MOD_ID, mod_api::ConfigKind::Server, server_config::DEFAULT_TEXT));
        } else {
            event.configs.push(mod_api::Config::new(sync::MOD_ID, mod_api::ConfigKind::Client, &default));
        }
    }

    /// The config file was read at startup.
    fn config_loading(&self, _side: mod_api::HostSide, event: &mut mod_api::ConfigLoadingEvent) {
        if event.config.mod_id == sync::MOD_ID && event.config.file == config_file::FILE_NAME {
            self.settings_sync.file_read(&event.config.text);
        }
        if event.config.mod_id == sync::MOD_ID && event.config.file == server_config::FILE_NAME {
            for warning in server_config::file_read(&event.config.text) {
                eprintln!("{}: {} {warning}", sync::MOD_ID, server_config::FILE_NAME);
            }
        }
    }

    /// The config file was saved while the game runs.
    fn config_reloading(&self, _side: mod_api::HostSide, event: &mut mod_api::ConfigReloadingEvent) {
        if event.config.mod_id == sync::MOD_ID && event.config.file == config_file::FILE_NAME {
            self.settings_sync.file_read(&event.config.text);
        }
        if event.config.mod_id == sync::MOD_ID && event.config.file == server_config::FILE_NAME {
            for warning in server_config::file_read(&event.config.text) {
                eprintln!("{}: {} {warning}", sync::MOD_ID, server_config::FILE_NAME);
            }
        }
    }

    /// Middle click toggles the lock. The click is swallowed only when it locks or
    /// unlocks, so middle-clicking a block with nothing to lock still picks it.
    /// (Left and right clicks while locked go to the target through `attack.rs`.)
    fn mouse_button_pre(&self, host: &mut dyn ClientHost, event: &mut mod_api::MouseButtonPreEvent) -> Flow {
        if event.button != input::MIDDLE_MOUSE_BUTTON || !event.pressed || !host.playing() {
            return Flow::Continue;
        }
        if !self.session.allowed() {
            if self.lock.is_locked() {
                self.lock.with(|state| state.release());
            }
            attack::direct_clicks_at(host, None);
            self.session.explain_once(host);
            return Flow::Continue;
        }
        if self.toggle_lock(host) {
            attack::direct_clicks_at(host, self.lock.target());
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// Scroll switches targets while locked (and doesn't change the hotbar slot).
    fn mouse_scrolling(&self, host: &mut dyn ClientHost, event: &mut mod_api::MouseScrollingEvent) -> Flow {
        if !self.session.allowed() {
            if self.lock.is_locked() {
                self.lock.with(|state| state.release());
            }
            attack::direct_clicks_at(host, None);
            return Flow::Continue;
        }
        if !self.lock.is_locked() {
            return Flow::Continue;
        }
        if let Some(side) = input::scroll_side(event.scroll[1]) {
            self.switch_target(host, side);
        }
        Flow::Cancel
    }

    /// While locked the lock steers the camera, so the mouse doesn't turn it.
    fn calculate_player_turn(&self, _host: &dyn ClientHost, event: &mut mod_api::CalculatePlayerTurnEvent) {
        if self.lock.is_locked() && self.session.allowed() {
            event.mouse_sensitivity = input::MUTED_MOUSE_SENSITIVITY;
            event.cinematic_camera_enabled = false; // or it replays buffered mouse motion
        }
    }

    fn client_tick_pre(&self, host: &mut dyn ClientHost, _event: &mut mod_api::ClientTickPreEvent) {
        self.settings_sync.copy_to_game_if_pending(host);
        self.session.update(host);
        if !self.session.allowed() {
            // Singleplayer only: no lock, no assist, clicks go where the crosshair is.
            if self.lock.is_locked() {
                self.lock.with(|state| state.release());
            }
            self.deaths.take();
        } else if host.playing()
            && let Some(view) = scan::player_view(host)
        {
            match self.lock.target() {
                Some(target) => {
                    if let Some(target) = self.handle_target_death(host, &view, target) {
                        self.update_lock(host, &view, target);
                    }
                }
                None => {
                    self.deaths.take();
                    self.assist(host, &view);
                }
            }
        }
        // Clicks follow the lock: to the target while locked, back to the crosshair after.
        attack::direct_clicks_at(host, self.lock.target());
    }

    /// Camera turning happens every frame, not every tick, so it's smooth at any frame rate.
    fn render_frame_pre(&self, host: &mut dyn ClientHost, event: &mut mod_api::RenderFramePreEvent) {
        let seconds = self.frame_clock.lock().map_or(0.0, |mut clock| clock.tick());
        if !self.session.allowed() && self.lock.is_locked() {
            self.lock.with(|state| state.release());
            attack::direct_clicks_at(host, None);
        }
        if self.lock.is_locked() && host.playing() && self.session.allowed() {
            self.turn_camera(host, event.world_partial_tick, seconds);
        }
    }

    /// While locked, what's drawn trails the player's real aim instead of jumping with it:
    /// - first person: the view eases after the aim (`view_lag.rs`);
    /// - third person from behind: the cinematic camera (`cinematic.rs`).
    /// Both ease in and out. Only the picture moves; the player still aims at the target.
    fn camera_pose(&self, host: &dyn ClientHost, event: &mut mod_api::CameraPoseEvent) {
        let seconds = self.camera_clock.lock().map_or(0.0, |mut clock| clock.tick());

        let lag_active = self.lock.is_locked() && self.session.allowed() && scan::first_person(host);
        let aim = [event.yaw, event.pitch];
        let [yaw, pitch] = self.view_lag.lock().map_or(aim, |mut lag| lag.update(lag_active, aim, seconds));
        event.yaw = yaw;
        event.pitch = pitch;

        let goal = if self.session.allowed() {
            self.cinematic_goal(host, event.partial)
        } else {
            None
        };
        let normal = cinematic::Pose { position: *event.position, yaw: event.yaw, pitch: event.pitch };
        let pose = self.cinematic.lock().map_or(normal, |mut cam| cam.update(goal, normal, seconds));
        if pose != normal {
            *event.position = pose.position;
            event.yaw = pose.yaw;
            event.pitch = pose.pitch;
        }
    }

    /// The FOV narrows as the cinematic camera blends in.
    fn compute_fov(&self, _host: &dyn ClientHost, event: &mut mod_api::ComputeFovEvent) {
        if !self.session.allowed() {
            return;
        }
        let blend = self.cinematic.lock().map_or(0.0, |cam| cam.blend());
        if blend > 0.0 {
            event.degrees *= 1.0 + (config::get().fov_scale - 1.0) * blend;
        }
    }

    /// Watch for mobs playing their death animation, so the lock can move on from a dead target.
    /// Never cancels: the mob is drawn as usual.
    fn render_living_pre(&self, host: &dyn ClientHost, event: &mut mod_api::RenderLivingPreEvent) -> Flow {
        if self.session.allowed() && self.lock.is_locked() && event.entity.death_time > 0 {
            if let Some(entity) = host.entity(event.entity.id) {
                self.deaths.note_dying(entity);
            }
        }
        Flow::Continue
    }

    /// The crosshair is hidden while locked; the lock dot takes its place.
    fn gui_layer_pre(&self, _host: &dyn ClientHost, event: &mut mod_api::GuiLayerPreEvent) -> Flow {
        let hide = self.session.allowed()
            && self.lock.is_locked()
            && config::get().hide_crosshair
            && event.layer == hud::CROSSHAIR_LAYER;
        if hide { Flow::Cancel } else { Flow::Continue }
    }

    /// A dot marks the locked target, at the same blended position the camera follows.
    fn gui_render_post(&self, host: &dyn ClientHost, event: &mut mod_api::GuiRenderPostEvent) {
        if event.gui_hidden || !self.session.allowed() || !self.lock.is_locked() || !config::get().lock_dot {
            return;
        }
        let partial_tick = host.camera().map_or(1.0, |camera| camera.partial_tick);
        if let Some((_, point)) = self.lock.with(|state| state.track.at(partial_tick)) {
            hud::draw_lock_dot(host, event.batch, point);
        }
    }
}

//! The lock itself: which entity is locked, and when the lock ends.

use std::sync::Mutex;

use mod_api::EntityRef;

use crate::config;
use crate::smooth::Track;

/// What the lock is doing right now.
#[derive(Clone, Copy, Debug, Default)]
pub struct LockState {
    /// The entity the camera follows, if any.
    pub target: Option<EntityRef>,
    /// Ticks in a row the target has been behind a wall.
    pub hidden_ticks: u32,
    /// Recent eye and target positions, for smooth per-frame turning.
    pub track: Track,
}

impl LockState {
    pub fn lock(&mut self, target: EntityRef) {
        *self = Self { target: Some(target), ..Self::default() };
    }

    /// Switch to another target, keeping the camera's motion continuous.
    pub fn switch_to(&mut self, target: EntityRef) {
        self.target = Some(target);
        self.hidden_ticks = 0;
    }

    pub fn release(&mut self) {
        *self = Self::default();
    }

    /// Record this tick's line of sight; returns false when the lock should end.
    pub fn track_visibility(&mut self, visible: bool) -> bool {
        self.hidden_ticks = if visible { 0 } else { self.hidden_ticks + 1 };
        self.hidden_ticks <= config::get().max_hidden_ticks()
    }
}

/// `LockState` shared between hooks (they take `&self`).
#[derive(Default)]
pub struct SharedLock(Mutex<LockState>);

impl SharedLock {
    pub fn with<R>(&self, f: impl FnOnce(&mut LockState) -> R) -> R {
        let mut state = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut state)
    }

    pub fn target(&self) -> Option<EntityRef> {
        self.with(|state| state.target)
    }

    pub fn is_locked(&self) -> bool {
        self.target().is_some()
    }
}

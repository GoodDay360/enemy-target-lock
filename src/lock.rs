//! The lock itself: which entity is locked, and when the lock ends.

use std::sync::Mutex;

use mod_api::EntityRef;

use crate::smooth::Track;

/// The lock is released when the target gets farther away than this (blocks).
pub const MAX_LOCK_DISTANCE: f64 = 32.0;
/// The lock is released after the target has been out of sight this many ticks (3 s).
pub const MAX_HIDDEN_TICKS: u32 = 60;

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
        self.hidden_ticks <= MAX_HIDDEN_TICKS
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

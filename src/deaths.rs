//! Which mobs are dying.
//!
//! The client can't read other mobs' health, but it does draw their death animation:
//! each frame, `render_living_pre` reports a `death_time` above 0 for a mob that has
//! died. [`DeathWatch`] collects those mobs between ticks, so the lock can move on as
//! soon as its target dies instead of waiting for the body to disappear.

use std::collections::HashSet;
use std::sync::Mutex;

use mod_api::EntityRef;

#[derive(Default)]
pub struct DeathWatch(Mutex<HashSet<EntityRef>>);

impl DeathWatch {
    /// A frame drew `entity` dying.
    pub fn note_dying(&self, entity: EntityRef) {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(entity);
    }

    /// Every mob seen dying since the last call. Call once per tick.
    pub fn take(&self) -> HashSet<EntityRef> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }
}

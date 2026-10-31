//! Bounded owner bindings for legacy MCP sessions.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAdmission {
    Bound,
    Existing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("session owner binding unavailable")]
pub struct SessionOwnerError;

#[derive(Debug)]
pub struct SessionOwners {
    entries: Mutex<HashMap<String, Entry>>,
    capacity: usize,
    ttl: Duration,
}

#[derive(Debug)]
struct Entry {
    owner: String,
    touched: Instant,
}

impl SessionOwners {
    #[must_use]
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            capacity: capacity.max(1),
            ttl,
        }
    }

    pub fn admit(&self, session: &str, owner: &str) -> Result<SessionAdmission, SessionOwnerError> {
        if session.is_empty() || owner.is_empty() {
            return Err(SessionOwnerError);
        }
        let mut entries = self.entries.lock().map_err(|_| SessionOwnerError)?;
        let now = Instant::now();
        entries.retain(|_, entry| now.duration_since(entry.touched) <= self.ttl);
        if let Some(entry) = entries.get_mut(session) {
            if entry.owner != owner {
                return Err(SessionOwnerError);
            }
            entry.touched = now;
            return Ok(SessionAdmission::Existing);
        }
        if entries.len() >= self.capacity
            && let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.touched)
                .map(|(session, _)| session.clone())
        {
            entries.remove(&oldest);
        }
        entries.insert(
            session.to_owned(),
            Entry {
                owner: owner.to_owned(),
                touched: now,
            },
        );
        Ok(SessionAdmission::Bound)
    }

    pub fn remove(&self, session: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(session);
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.lock().map_or(0, |entries| entries.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

static OWNERS: OnceLock<SessionOwners> = OnceLock::new();

#[must_use]
pub fn global() -> &'static SessionOwners {
    OWNERS.get_or_init(|| SessionOwners::new(256, Duration::from_mins(30)))
}

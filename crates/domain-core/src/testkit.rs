//! Test doubles for the pure core's ports: an in-memory single-writer [`Repository`] and a
//! controllable [`FakeClock`]. These let the core be unit- and property-tested with no I/O
//! (NFR-1.1.3). The real persistence (Diesel, schema-per-tenant) lives in SQUIRE-S-0002.

use crate::contract::*;

/// In-memory `Repository`: upserts definitions by id, appends events, and keeps a small audit
/// trail of the `by` actor for `Put*`/`SetXActive` (the real store keeps last-editor columns —
/// ADR SQUIRE-A-0007; this is a lightweight stand-in for tests).
#[derive(Clone, Debug, Default)]
pub struct InMemoryRepository {
    pub users: Vec<User>,
    pub quests: Vec<Quest>,
    pub items: Vec<RedeemableItem>,
    pub achievements: Vec<Achievement>,
    pub events: Vec<Event>,
    pub audit: Vec<AuditEntry>,
}

/// A recorded definition write, for asserting authoring-audit behaviour in tests.
#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub by: Option<UserId>,
    pub what: String,
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience for tests: seed a definition/user directly (no audit) before exercising
    /// `handle`. Equivalent to applying the corresponding `Put*` change with `by = None`.
    pub fn seed(&mut self, changes: &[Change]) {
        let _ = self.apply(None, changes);
    }
}

impl Repository for InMemoryRepository {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            users: self.users.clone(),
            quests: self.quests.clone(),
            items: self.items.clone(),
            achievements: self.achievements.clone(),
            events: self.events.clone(),
        }
    }

    fn apply(&mut self, by: Option<UserId>, changes: &[Change]) -> Result<(), RepoError> {
        for change in changes {
            match change {
                Change::Append(e) => self.events.push(e.clone()),
                Change::PutQuest(q) => {
                    upsert(&mut self.quests, q.clone(), |x| x.id == q.id);
                    self.audit.push(AuditEntry {
                        by,
                        what: format!("PutQuest({:?})", q.id),
                    });
                }
                Change::PutItem(i) => {
                    upsert(&mut self.items, i.clone(), |x| x.id == i.id);
                    self.audit.push(AuditEntry {
                        by,
                        what: format!("PutItem({:?})", i.id),
                    });
                }
                Change::PutAchievement(a) => {
                    upsert(&mut self.achievements, a.clone(), |x| x.id == a.id);
                    self.audit.push(AuditEntry {
                        by,
                        what: format!("PutAchievement({:?})", a.id),
                    });
                }
                Change::PutUser(u) => {
                    upsert(&mut self.users, u.clone(), |x| x.id == u.id);
                    self.audit.push(AuditEntry {
                        by,
                        what: format!("PutUser({:?})", u.id),
                    });
                }
                Change::SetQuestActive(id, active) => {
                    if let Some(q) = self.quests.iter_mut().find(|x| x.id == *id) {
                        q.active = *active;
                    }
                    self.audit.push(AuditEntry {
                        by,
                        what: format!("SetQuestActive({:?},{})", id, active),
                    });
                }
                Change::SetItemActive(id, active) => {
                    if let Some(i) = self.items.iter_mut().find(|x| x.id == *id) {
                        i.active = *active;
                    }
                    self.audit.push(AuditEntry {
                        by,
                        what: format!("SetItemActive({:?},{})", id, active),
                    });
                }
                Change::SetAchievementActive(id, active) => {
                    if let Some(a) = self.achievements.iter_mut().find(|x| x.id == *id) {
                        a.active = *active;
                    }
                    self.audit.push(AuditEntry {
                        by,
                        what: format!("SetAchievementActive({:?},{})", id, active),
                    });
                }
                Change::SetUserActive(id, active) => {
                    if let Some(u) = self.users.iter_mut().find(|x| x.id == *id) {
                        u.active = *active;
                    }
                    self.audit.push(AuditEntry {
                        by,
                        what: format!("SetUserActive({:?},{})", id, active),
                    });
                }
            }
        }
        Ok(())
    }
}

fn upsert<T>(v: &mut Vec<T>, item: T, matches: impl Fn(&T) -> bool) {
    if let Some(slot) = v.iter_mut().find(|x| matches(x)) {
        *slot = item;
    } else {
        v.push(item);
    }
}

/// Deterministic, controllable `Clock` for tests (NFR-1.1.1: a single configured "today").
#[derive(Clone, Copy, Debug)]
pub struct FakeClock {
    pub today: Date,
    pub now: Timestamp,
}

impl FakeClock {
    pub fn at(today: Date, now: Timestamp) -> Self {
        Self { today, now }
    }
}

impl Clock for FakeClock {
    fn today(&self) -> Date {
        self.today
    }
    fn now(&self) -> Timestamp {
        self.now
    }
}

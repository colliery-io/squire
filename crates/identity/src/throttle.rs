//! Login throttling (SQUIRE-T-0130): per-account lockout with exponential backoff.
//!
//! Secrets are Argon2id-hashed, which prices each guess but does not bound how many an attacker may
//! make — and Squire's attacker is *on the network by design* (a Squire's own device shares the LAN
//! and the tailnet with the api and, when the operator exposes it, the Keep). Household secrets are
//! chosen by parents for convenience, so the guess budget is the control that matters.
//!
//! **Policy.** The first [`FREE_FAILURES`] consecutive wrong secrets for an account cost nothing (a
//! parent fat-fingering a password never notices this module). Each further failure locks the
//! account for [`BASE_LOCK_MS`], doubling per failure, capped at [`MAX_LOCK_MS`]. While locked, login
//! is refused **without testing the secret** — a correct guess during a lock does not succeed, or
//! the lock would bound nothing. A success clears the account; so does [`RESET_AFTER_MS`] of quiet.
//! Steady state that is ≈100–120 guesses/day/account, versus ~10⁶ unthrottled.
//!
//! **Accepted trade-off — lockout as a nuisance.** Keyed by account (the identity port sees no
//! source address), so a Squire can keep a Knight's *login* locked by failing on purpose. That is
//! loud (a `warn` per refusal names the account), capped at 15 min, and low-stakes here: tokens and
//! Keep sessions are long-lived, so an already-signed-in Knight is unaffected and only a fresh login
//! waits. Per-source keying would need the peer address threaded through the port; not worth it yet.
//!
//! **Bounded memory.** The caller records failures only for accounts that *have a credential*, so
//! the map is bounded by household membership — arbitrary `UserId`s in requests allocate nothing.
//! The cost is a member-enumeration oracle (only real accounts ever lock), which is a non-secret in
//! a family app: the Squires know the Knights' names, and the Keep already accepts display names.
//!
//! State is in-memory: a restart clears it, and restarts are not attacker-controllable.
//!
//! Not internally synchronized. [`ProdIdentity`](crate::ProdIdentity) holds it in a `Mutex` and keeps
//! that lock across *check → verify → record*, so concurrent attempts cannot all pass the check
//! before the first failure lands (which would turn every lock window into an unbounded burst).

use std::collections::HashMap;

/// Consecutive failures that cost nothing.
pub const FREE_FAILURES: u32 = 5;
/// Lock after the first failure beyond [`FREE_FAILURES`]; doubles with each further one.
pub const BASE_LOCK_MS: i64 = 30_000;
/// Longest single lock.
pub const MAX_LOCK_MS: i64 = 15 * 60_000;
/// A failure streak older than this is forgotten (so a typo last month isn't held against anyone).
pub const RESET_AFTER_MS: i64 = 60 * 60_000;

#[derive(Clone, Copy, Debug, Default)]
struct Entry {
    failures: u32,
    last_failure_ms: i64,
    locked_until_ms: i64,
}

/// Per-account failed-login bookkeeping. Keys are `(household handle, user id)`.
#[derive(Debug, Default)]
pub struct LoginThrottle {
    entries: HashMap<(String, u128), Entry>,
}

impl LoginThrottle {
    /// `Ok` if `key` may attempt a login at `now_ms`; else `Err(seconds until it may)` (≥ 1).
    pub fn check(&self, key: &(String, u128), now_ms: i64) -> Result<(), u32> {
        match self.entries.get(key) {
            Some(e) if now_ms < e.locked_until_ms => {
                let remaining_ms = e.locked_until_ms - now_ms;
                Err(((remaining_ms + 999) / 1000).max(1) as u32)
            }
            _ => Ok(()),
        }
    }

    /// Record a wrong secret for `key`, locking the account once past the free allowance.
    pub fn record_failure(&mut self, key: (String, u128), now_ms: i64) {
        let e = self.entries.entry(key).or_default();
        if now_ms - e.last_failure_ms > RESET_AFTER_MS {
            e.failures = 0;
        }
        e.failures = e.failures.saturating_add(1);
        e.last_failure_ms = now_ms;
        if e.failures >= FREE_FAILURES {
            // 30s, 60s, 120s, … — the shift is clamped so it cannot overflow before the cap applies.
            let doublings = (e.failures - FREE_FAILURES).min(20);
            let lock_ms = (BASE_LOCK_MS << doublings).min(MAX_LOCK_MS);
            e.locked_until_ms = now_ms + lock_ms;
        }
    }

    /// A correct secret clears the account's streak.
    pub fn record_success(&mut self, key: &(String, u128)) {
        self.entries.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> (String, u128) {
        ("home".to_string(), 1)
    }

    #[test]
    fn free_failures_never_lock() {
        let mut t = LoginThrottle::default();
        for i in 0..(FREE_FAILURES - 1) {
            t.record_failure(key(), 1_000 + i as i64);
            assert_eq!(t.check(&key(), 1_000 + i as i64), Ok(()));
        }
    }

    #[test]
    fn locks_after_the_allowance_then_doubles_to_the_cap() {
        let mut t = LoginThrottle::default();
        let mut now = 1_000;
        for _ in 0..FREE_FAILURES {
            t.record_failure(key(), now);
        }
        assert_eq!(t.check(&key(), now), Err(30), "5th failure → 30s");
        assert_eq!(t.check(&key(), now + 29_001), Err(1), "rounds up, never 0");
        assert_eq!(t.check(&key(), now + 30_000), Ok(()), "lock expires");

        now += 30_000;
        t.record_failure(key(), now);
        assert_eq!(t.check(&key(), now), Err(60), "6th → doubled");

        // Keep failing the instant each lock lapses: the lock grows to the cap and stays there.
        for _ in 0..40 {
            now += MAX_LOCK_MS;
            t.record_failure(key(), now);
        }
        assert_eq!(t.check(&key(), now), Err((MAX_LOCK_MS / 1000) as u32));
    }

    #[test]
    fn success_clears_the_streak() {
        let mut t = LoginThrottle::default();
        for _ in 0..(FREE_FAILURES - 1) {
            t.record_failure(key(), 1_000);
        }
        t.record_success(&key());
        t.record_failure(key(), 2_000);
        assert_eq!(t.check(&key(), 2_000), Ok(()), "streak restarted from zero");
    }

    #[test]
    fn a_quiet_hour_forgets_the_streak() {
        let mut t = LoginThrottle::default();
        for _ in 0..(FREE_FAILURES - 1) {
            t.record_failure(key(), 1_000);
        }
        let later = 1_000 + RESET_AFTER_MS + 1;
        t.record_failure(key(), later);
        assert_eq!(
            t.check(&key(), later),
            Ok(()),
            "old failures no longer count"
        );
    }

    #[test]
    fn accounts_are_independent() {
        let mut t = LoginThrottle::default();
        for _ in 0..FREE_FAILURES {
            t.record_failure(key(), 1_000);
        }
        assert!(t.check(&key(), 1_000).is_err());
        assert_eq!(t.check(&("home".to_string(), 2), 1_000), Ok(()));
        assert_eq!(t.check(&("other".to_string(), 1), 1_000), Ok(()));
    }
}

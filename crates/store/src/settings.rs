//! Household configuration runtime (ADR SQUIRE-A-0011): the hot, lock-free config cell and the
//! timezone-aware [`LocalClock`].
//!
//! The live config is a **forward projection** kept current by the single-writer path (AR-1): every
//! `set_setting` write hot-swaps [`LiveConfig`], so readers (the clock, the Keep/api handlers)
//! always see the DB's current value via a lock-free `.load()` — no polling, nothing out-of-band to
//! drift from.

use std::sync::Arc;

use arc_swap::ArcSwap;
use domain_core::contract::{Clock, Date, HouseholdConfig, Timestamp};

use crate::{now_millis, UNIX_TO_MONDAY_EPOCH_OFFSET};

/// A resolved config snapshot: the typed view plus the **pre-parsed** timezone for the clock's hot
/// path (so `today()` never re-parses the IANA string).
#[derive(Clone)]
pub struct ConfigView {
    /// The typed settings view (what handlers read).
    pub config: HouseholdConfig,
    /// The parsed household timezone (what the clock reads).
    pub tz: jiff::tz::TimeZone,
}

impl ConfigView {
    /// Resolve a [`HouseholdConfig`] into a view, parsing its timezone with a **UTC fallback** on an
    /// unknown/invalid name (never panics).
    pub fn resolve(config: HouseholdConfig) -> Self {
        let tz =
            jiff::tz::TimeZone::get(&config.timezone).unwrap_or_else(|_| jiff::tz::TimeZone::UTC);
        Self { config, tz }
    }
}

impl Default for ConfigView {
    fn default() -> Self {
        Self::resolve(HouseholdConfig::default())
    }
}

/// The shared, hot, lock-free household-config cell. Readers `.load()` a snapshot (a cheap atomic
/// load + `Arc` clone, never a lock); the single writer hot-swaps it on every config change.
pub type LiveConfig = Arc<ArcSwap<ConfigView>>;

/// Build a new live cell seeded with `config`.
pub fn live_config(config: HouseholdConfig) -> LiveConfig {
    Arc::new(ArcSwap::from_pointee(ConfigView::resolve(config)))
}

/// Whether `name` resolves to a real IANA timezone (ADR A-0011). Used by onboarding (#3) and the
/// Keep Settings handler (#4) to validate a candidate zone *before* persisting it, so the stored
/// value is always meaningful rather than silently falling back to UTC at clock-build time.
pub fn valid_timezone(name: &str) -> bool {
    jiff::tz::TimeZone::get(name).is_ok()
}

/// Convert unix `millis` to the domain's Monday-aligned [`Date`] **in `tz`** (DST-correct). Agrees
/// with the UTC `date_from_unix_millis` when `tz` is UTC.
pub fn date_in_zone(millis: i64, tz: &jiff::tz::TimeZone) -> Date {
    let ts = jiff::Timestamp::from_millisecond(millis).unwrap_or_default();
    let civil = ts.to_zoned(tz.clone()).date();
    let days = days_from_civil(
        civil.year() as i64,
        civil.month() as i64,
        civil.day() as i64,
    );
    Date((days + UNIX_TO_MONDAY_EPOCH_OFFSET) as i32)
}

/// Days since 1970-01-01 for a proleptic-Gregorian civil date (Howard Hinnant's algorithm). Pure
/// integer math — no tz, no jiff date-diff API — so the day-count convention lives in one place.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// Timezone-aware wall clock (ADR A-0011). `now()` is unix millis (UTC, unchanged); `today()` is the
/// Monday-aligned day-count in the household's **live** timezone (a lock-free read of the cell), so
/// daily/weekly quests reset at the family's local midnight.
#[derive(Clone)]
pub struct LocalClock {
    live: LiveConfig,
}

impl LocalClock {
    /// Build a clock over the shared live-config cell.
    pub fn new(live: LiveConfig) -> Self {
        Self { live }
    }

    /// The shared cell this clock reads — callers hot-swap it on a config change so `today()`
    /// reflects the new timezone immediately.
    pub fn live(&self) -> &LiveConfig {
        &self.live
    }
}

impl Clock for LocalClock {
    fn today(&self) -> Date {
        date_in_zone(now_millis(), &self.live.load().tz)
    }

    fn now(&self) -> Timestamp {
        Timestamp(now_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tz(name: &str) -> jiff::tz::TimeZone {
        jiff::tz::TimeZone::get(name).unwrap()
    }

    // 2026-01-15 07:30 UTC is 2026-01-14 23:30 in America/Los_Angeles — the local date is the
    // *previous* day. The zone-aware conversion must return the LA calendar day, not the UTC one.
    #[test]
    fn date_in_zone_respects_local_midnight() {
        // 2026-01-15T07:30:00Z in unix millis.
        let millis = jiff::civil::date(2026, 1, 15)
            .at(7, 30, 0, 0)
            .to_zoned(tz("UTC"))
            .unwrap()
            .timestamp()
            .as_millisecond();
        let utc_day = date_in_zone(millis, &tz("UTC"));
        let la_day = date_in_zone(millis, &tz("America/Los_Angeles"));
        // LA is one calendar day behind at this instant.
        assert_eq!(utc_day.0 - la_day.0, 1);
    }

    // The zone-aware conversion in UTC must agree with the legacy UTC day-count exactly.
    #[test]
    fn utc_agrees_with_legacy() {
        for millis in [
            0i64,
            1_000_000_000_000,
            1_781_000_000_000,
            1_900_000_000_000,
        ] {
            assert_eq!(
                date_in_zone(millis, &tz("UTC")),
                crate::date_from_unix_millis(millis)
            );
        }
    }

    // An unknown zone string falls back to UTC rather than panicking.
    #[test]
    fn bad_zone_falls_back_to_utc() {
        let view = ConfigView::resolve(HouseholdConfig {
            timezone: "Not/AZone".into(),
        });
        assert_eq!(view.tz, jiff::tz::TimeZone::UTC);
    }
}

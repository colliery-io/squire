//! Household configuration (ADR SQUIRE-A-0011).
//!
//! Per-tenant settings, modelled as a **typed view** over a key/value `config` table in the store.
//! The first setting is the household `timezone`; the struct grows as settings accrue (week-start,
//! display name, quiet hours, …). Defaults are the safe baseline used when a key is absent or its
//! stored value is unparseable, so the in-memory view never fails to assemble.

/// The typed, in-memory view of a household's settings (assembled from the KV `config` rows).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseholdConfig {
    /// IANA timezone name (e.g. `"America/Los_Angeles"`). Sets the household's local-midnight day
    /// boundary for daily/weekly quests. Defaults to `"UTC"`.
    pub timezone: String,
    /// The household's **waking window** in minutes since local midnight (SQUIRE-T-0138): a squire's
    /// phone stays silent outside it. Defaults to 07:00–20:00. Applies to a squire's notifications
    /// only — a Knight is an adult with their own do-not-disturb (see SQUIRE-I-0007).
    #[cfg_attr(feature = "serde", serde(default = "default_wake_from"))]
    pub notify_wake_from: u16,
    #[cfg_attr(feature = "serde", serde(default = "default_wake_to"))]
    pub notify_wake_to: u16,
    /// **Chore times** (SQUIRE-T-0140): minutes since local midnight at which a squire's phone
    /// reminds them what is left — e.g. 16:30 after school and 19:00 last call. Empty = no nudge,
    /// which is the default: a household opts in rather than being nagged out of the box.
    #[cfg_attr(feature = "serde", serde(default))]
    pub chore_times: Vec<u16>,
}

/// At most this many nudges a day. Two is a nudge; five is nagging, and a child stops reading them.
pub const MAX_CHORE_TIMES: usize = 2;

fn default_wake_from() -> u16 {
    7 * 60
}
fn default_wake_to() -> u16 {
    20 * 60
}

impl Default for HouseholdConfig {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            notify_wake_from: default_wake_from(),
            notify_wake_to: default_wake_to(),
            chore_times: Vec::new(),
        }
    }
}

/// A minute-of-day is `0..=1439`. Used to validate the waking window before it is persisted.
pub fn valid_minute_of_day(m: u16) -> bool {
    m < 24 * 60
}

/// The string keys under which settings are persisted in the KV `config` table. Keeping them in one
/// place keeps the store accessors and the typed-view assembly in sync.
pub mod config_keys {
    /// IANA timezone name. See [`super::HouseholdConfig::timezone`].
    pub const TIMEZONE: &str = "timezone";
    /// Waking window, minutes since local midnight (SQUIRE-T-0138).
    pub const NOTIFY_WAKE_FROM: &str = "notify_wake_from";
    pub const NOTIFY_WAKE_TO: &str = "notify_wake_to";
    /// Chore times, comma-separated minutes since local midnight (SQUIRE-T-0140).
    pub const CHORE_TIMES: &str = "chore_times";

    /// A squire's chosen **tincture** — the colour their screens wear (SQUIRE-T-0136). One key per
    /// member, in the household config table, so no domain-model change: `tincture:<user id>`.
    pub fn tincture(user: crate::contract::UserId) -> String {
        format!("tincture:{}", user.0)
    }
}

/// The tinctures a squire may choose from — heraldry's colours, plus sable. Closed set: the apps map
/// each name to a palette, so an unknown value would render nothing. [`DEFAULT_TINCTURE`] is what an
/// unset squire wears.
pub const TINCTURES: [&str; 6] = ["gules", "azure", "vert", "purpure", "tenne", "sable"];
pub const DEFAULT_TINCTURE: &str = "vert";

/// Normalise a chosen tincture: trimmed + lowercased, and only if it is one of [`TINCTURES`].
pub fn valid_tincture(s: &str) -> Option<&'static str> {
    let s = s.trim().to_ascii_lowercase();
    TINCTURES.iter().copied().find(|t| *t == s)
}

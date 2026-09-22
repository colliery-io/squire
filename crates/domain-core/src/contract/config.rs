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
}

impl Default for HouseholdConfig {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
        }
    }
}

/// The string keys under which settings are persisted in the KV `config` table. Keeping them in one
/// place keeps the store accessors and the typed-view assembly in sync.
pub mod config_keys {
    /// IANA timezone name. See [`super::HouseholdConfig::timezone`].
    pub const TIMEZONE: &str = "timezone";

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

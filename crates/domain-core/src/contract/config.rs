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
}

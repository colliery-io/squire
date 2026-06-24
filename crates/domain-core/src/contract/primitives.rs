// (No sibling imports needed — primitives is the leaf module.)

// ─── Primitives (placeholders) ──────────────────────────────────────────────
pub type Points = u32;

// ─── Currency (SQUIRE-A-0013 / SQUIRE-T-0097) ───────────────────────────────
/// A household currency. `Coins` is the original in-app currency (folded in as the first member,
/// SQUIRE-A-0013); `Cash` is real-world dollars. Adding a currency is a new member here + its
/// [`CurrencyPolicy`] row + UI strings — no engine or projection changes. The serde tag is the
/// stable wire/storage identity (also used as the `events.currency` column value), so member NAMES
/// must never change once shipped.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Currency {
    /// In-app coins — earned from quests, spent in the reward shop, award achievement bonuses.
    Coins,
    /// Real-world money in **whole dollars** — an IOU the parent settles by paying out; never
    /// spendable in the in-app shop.
    Cash,
}

impl Currency {
    /// The stable string tag used in storage (`events.currency`) and back-compat defaults. Must match
    /// the serde representation and never change for a shipped member.
    pub fn tag(self) -> &'static str {
        match self {
            Currency::Coins => "Coins",
            Currency::Cash => "Cash",
        }
    }

    /// Parse a [`Currency`] from its [`tag`](Self::tag); `None` for an unknown tag.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "Coins" => Some(Currency::Coins),
            "Cash" => Some(Currency::Cash),
            _ => None,
        }
    }

    /// Every currency, in a stable order — for enumerating balances / catalogs.
    pub fn all() -> &'static [Currency] {
        &[Currency::Coins, Currency::Cash]
    }

    /// This currency's behavior, expressed as **data** (no `if currency == Coins` branches in the
    /// engine). See [`CurrencyPolicy`].
    pub fn policy(self) -> CurrencyPolicy {
        match self {
            Currency::Coins => CurrencyPolicy {
                name: "coins",
                symbol: "🪙",
                spendable_in_shop: true,
                supports_payout: false,
                earns_achievement_bonus: true,
                floors_at_zero: true,
            },
            Currency::Cash => CurrencyPolicy {
                name: "dollars",
                symbol: "$",
                spendable_in_shop: false,
                supports_payout: true,
                earns_achievement_bonus: false,
                floors_at_zero: true,
            },
        }
    }
}

/// The policy that distinguishes one currency from another — kept as data so the engine stays
/// currency-agnostic (SQUIRE-A-0013). Amounts are always **whole units** (coins / whole dollars).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurrencyPolicy {
    /// Display name (e.g. "coins", "dollars").
    pub name: &'static str,
    /// Display symbol/glyph (e.g. "🪙", "$").
    pub symbol: &'static str,
    /// Can it price + buy rewards in the shop? (Coins: yes; dollars: no.)
    pub spendable_in_shop: bool,
    /// Can a parent settle/pay it out (a negative adjust representing a real-world payment)?
    pub supports_payout: bool,
    /// Do achievement bonuses pay out in this currency? (Coins: yes; dollars: no, initially.)
    pub earns_achievement_bonus: bool,
    /// Is the balance floored at zero (no debt)?
    pub floors_at_zero: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Date(pub i32); //   days since an epoch
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Timestamp(pub i64); // unix millis

// The `u128` id newtypes are schema-typed `int64` (A-0009 invariant): ids stay numeric on the
// wire (runtime serde is unchanged — still `u128`), but the schema advertises `integer`/`int64`
// since every minted id fits in i64 with room to spare.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = i64))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct QuestId(pub u128);
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = i64))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ItemId(pub u128);
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = i64))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AchievementId(pub u128);
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = i64))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ClaimId(pub u128);
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = i64))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RequestId(pub u128);
/// Client-minted idempotency key for privileged commands that lack a natural one
/// (`RedeemItem` direct, `AdjustPoints`). The parent phone's offline outbox mints it
/// once and reuses it on every retry; the engine dedupes by finding an emitted event
/// that already carries it, so retry-safety stays *derived from the append-only log*
/// — the same mechanism as `claim_id` / `request_id`. See ADR SQUIRE-A-0001.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = i64))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CommandId(pub u128);

/// Identifies a household member. A household has one-or-more Knights (adult/parent) and
/// Squires (child/player) — counts are fixed nowhere. Player activity is attributed to a
/// Squire via this id. See ADR SQUIRE-A-0004. (Tenancy is schema-level and absent from
/// these types — ADR SQUIRE-A-0002.)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = i64))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UserId(pub u128);

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Role {
    Knight,
    Squire,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

/// Free-form text labels — the parent types whatever grouping they like.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Category(pub String);

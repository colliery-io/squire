// (No sibling imports needed — primitives is the leaf module.)

// ─── Primitives (placeholders) ──────────────────────────────────────────────
pub type Points = u32;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Date(pub i32); //   days since an epoch
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Timestamp(pub i64); // unix millis

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct QuestId(pub u128);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct ItemId(pub u128);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct AchievementId(pub u128);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct ClaimId(pub u128);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct RequestId(pub u128);
/// Client-minted idempotency key for privileged commands that lack a natural one
/// (`RedeemItem` direct, `AdjustPoints`). The parent phone's offline outbox mints it
/// once and reuses it on every retry; the engine dedupes by finding an emitted event
/// that already carries it, so retry-safety stays *derived from the append-only log*
/// — the same mechanism as `claim_id` / `request_id`. See ADR SQUIRE-A-0001.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct CommandId(pub u128);

/// Identifies a household member. A household has one-or-more Knights (adult/parent) and
/// Squires (child/player) — counts are fixed nowhere. Player activity is attributed to a
/// Squire via this id. See ADR SQUIRE-A-0004. (Tenancy is schema-level and absent from
/// these types — ADR SQUIRE-A-0002.)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)] pub struct UserId(pub u128);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Role { Knight, Squire }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Weekday { Mon, Tue, Wed, Thu, Fri, Sat, Sun }

/// Free-form text labels — the parent types whatever grouping they like.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Category(pub String);

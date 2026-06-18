//! Hand-written Diesel table definitions matching `migrations/2026-06-16-000000_init`.
//!
//! Type mapping notes (portable SQLite + Postgres):
//!   * Ids (`UserId`/`QuestId`/…) are stored as `Text` (the u128 rendered as a decimal
//!     string) so the same DDL is valid on both backends.
//!   * Booleans (`active`, `auto_approve`, …) are `Integer` (0/1) for portability.
//!   * `Points` / `Date` / `Timestamp` are `BigInt`.
//!   * `events.seq` is the append-only order key. The column is declared
//!     `INTEGER PRIMARY KEY AUTOINCREMENT`; on SQLite this is a 64-bit rowid alias, so
//!     we map it as `BigInt`.

diesel::table! {
    users (id) {
        id -> Text,
        role -> Text,
        display_name -> Text,
        active -> Integer,
        created_by -> Nullable<Text>,
        created_at -> BigInt,
        updated_by -> Nullable<Text>,
        updated_at -> BigInt,
    }
}

diesel::table! {
    quests (id) {
        id -> Text,
        title -> Text,
        description -> Nullable<Text>,
        category -> Nullable<Text>,
        reward -> BigInt,
        cadence_kind -> Text,
        cadence_due -> Nullable<BigInt>,
        cadence_weekdays -> Nullable<Text>,
        cadence_n -> Nullable<BigInt>,
        cadence_anchor -> Nullable<BigInt>,
        assignment_kind -> Text,
        assignment_squires -> Nullable<Text>,
        completion -> Text,
        auto_approve -> Integer,
        repeatable_within_day -> Integer,
        icon -> Nullable<Text>,
        active -> Integer,
        created_by -> Nullable<Text>,
        created_at -> BigInt,
        updated_by -> Nullable<Text>,
        updated_at -> BigInt,
    }
}

diesel::table! {
    items (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        cost -> BigInt,
        gate -> Nullable<Text>,
        availability -> Text,
        icon -> Nullable<Text>,
        active -> Integer,
        created_by -> Nullable<Text>,
        created_at -> BigInt,
        updated_by -> Nullable<Text>,
        updated_at -> BigInt,
    }
}

diesel::table! {
    achievements (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        criterion_kind -> Text,
        scope_kind -> Nullable<Text>,
        scope_quest -> Nullable<Text>,
        scope_category -> Nullable<Text>,
        streak_length -> Nullable<BigInt>,
        streak_basis -> Nullable<Text>,
        total_count -> Nullable<BigInt>,
        points_total -> Nullable<BigInt>,
        bonus_points -> BigInt,
        active -> Integer,
        created_by -> Nullable<Text>,
        created_at -> BigInt,
        updated_by -> Nullable<Text>,
        updated_at -> BigInt,
    }
}

diesel::table! {
    events (seq) {
        seq -> BigInt,
        kind -> Text,
        squire -> Text,
        actor -> Nullable<Text>,
        at -> BigInt,
        claim_id -> Nullable<Text>,
        quest_id -> Nullable<Text>,
        on_date -> Nullable<BigInt>,
        points -> Nullable<BigInt>,
        amount -> Nullable<BigInt>,
        reason -> Nullable<Text>,
        request_id -> Nullable<Text>,
        command_id -> Nullable<Text>,
        item_id -> Nullable<Text>,
        achievement_id -> Nullable<Text>,
    }
}

diesel::table! {
    credentials (user_id) {
        user_id -> Text,
        secret_hash -> Text,
    }
}

diesel::table! {
    pairing_codes (code_hash) {
        code_hash -> Text,
        user_id -> Text,
        role -> Text,
        expires_at -> BigInt,
    }
}

diesel::table! {
    // Household configuration KV (ADR SQUIRE-A-0011): long, not wide — one row per setting.
    config (key) {
        key -> Text,
        value -> Text,
        updated_by -> Nullable<Text>,
        updated_at -> BigInt,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    users,
    quests,
    items,
    achievements,
    events,
);

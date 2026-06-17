-- Squire store — initial schema (SQUIRE-T-0008)
--
-- Portable DDL: works on both SQLite and Postgres. Only TEXT / INTEGER / BIGINT
-- column types and plain CREATE TABLE / CREATE INDEX are used. No backend-specific
-- types (no SERIAL, no UUID, no BOOLEAN — booleans are INTEGER 0/1; ids are TEXT
-- holding the u128 decimal string; timestamps/dates are BIGINT).
--
-- IDENTITY + DEFINITIONS are mutable tables. ACTIVITY (events) is an append-only log.
--
-- Audit columns (created_by / created_at / updated_by / updated_at) are stamped on
-- the definition + identity rows from Repository::apply's `by` argument (ADR A-0007).
-- `*_by` hold a UserId as TEXT and are nullable (NULL = system/seed).

-- ─── IDENTITY ───────────────────────────────────────────────────────────────
CREATE TABLE users (
    id           TEXT    NOT NULL PRIMARY KEY,  -- UserId (u128 as decimal string)
    role         TEXT    NOT NULL,              -- 'Knight' | 'Squire'
    display_name TEXT    NOT NULL,
    active       INTEGER NOT NULL,              -- 0/1 boolean
    created_by   TEXT,
    created_at   BIGINT  NOT NULL,
    updated_by   TEXT,
    updated_at   BIGINT  NOT NULL
);

-- ─── DEFINITIONS ────────────────────────────────────────────────────────────

-- Quest.
--   cadence_kind: 'OneOff' | 'Daily' | 'Weekly' | 'EveryNDays'
--   cadence_due       : Date (BIGINT) — OneOff only, nullable
--   cadence_weekdays  : TEXT — Weekly only; comma-delimited 'Mon,Wed,Fri' (set as delimited TEXT)
--   cadence_n / cadence_anchor : EveryNDays only
--   assignment_kind: 'AllSquires' | 'Squires'
--   assignment_squires: TEXT — Squires only; comma-delimited UserId decimal strings
--   completion: 'EachAssignee' | 'Race'
CREATE TABLE quests (
    id                   TEXT    NOT NULL PRIMARY KEY,
    title                TEXT    NOT NULL,
    description          TEXT,
    category             TEXT,                      -- Category label, nullable
    reward               BIGINT  NOT NULL,          -- Points
    cadence_kind         TEXT    NOT NULL,
    cadence_due          BIGINT,
    cadence_weekdays     TEXT,
    cadence_n            BIGINT,
    cadence_anchor       BIGINT,
    assignment_kind      TEXT    NOT NULL,
    assignment_squires   TEXT,
    completion           TEXT    NOT NULL,
    auto_approve         INTEGER NOT NULL,
    repeatable_within_day INTEGER NOT NULL,
    icon                 TEXT,
    active               INTEGER NOT NULL,
    created_by           TEXT,
    created_at           BIGINT  NOT NULL,
    updated_by           TEXT,
    updated_at           BIGINT  NOT NULL
);

-- RedeemableItem.
--   availability: 'Once' | 'Repeatable'
--   gate: AchievementId as TEXT, nullable
CREATE TABLE items (
    id           TEXT    NOT NULL PRIMARY KEY,
    name         TEXT    NOT NULL,
    description  TEXT,
    cost         BIGINT  NOT NULL,                  -- Points
    gate         TEXT,                              -- AchievementId, nullable
    availability TEXT    NOT NULL,
    icon         TEXT,
    active       INTEGER NOT NULL,
    created_by   TEXT,
    created_at   BIGINT  NOT NULL,
    updated_by   TEXT,
    updated_at   BIGINT  NOT NULL
);

-- Achievement.
--   criterion_kind: 'Streak' | 'TotalCompletions' | 'PointsEarned'
--   scope_kind: 'Quest' | 'Category' | 'Any'  (Streak + TotalCompletions only)
--   scope_quest: QuestId TEXT (scope_kind='Quest'); scope_category: TEXT (scope_kind='Category')
--   streak_length / streak_basis : Streak only ('ScheduledOccurrences' | 'CalendarDays')
--   total_count : TotalCompletions only
--   points_total: PointsEarned only
CREATE TABLE achievements (
    id            TEXT    NOT NULL PRIMARY KEY,
    name          TEXT    NOT NULL,
    description   TEXT,
    criterion_kind TEXT   NOT NULL,
    scope_kind    TEXT,
    scope_quest   TEXT,
    scope_category TEXT,
    streak_length BIGINT,
    streak_basis  TEXT,
    total_count   BIGINT,
    points_total  BIGINT,
    bonus_points  BIGINT  NOT NULL,
    active        INTEGER NOT NULL,
    created_by    TEXT,
    created_at    BIGINT  NOT NULL,
    updated_by    TEXT,
    updated_at    BIGINT  NOT NULL
);

-- ─── ACTIVITY (append-only event log) ───────────────────────────────────────
--
-- Layout: one row per Event, discriminated by `kind`, with the UNION of all Event
-- variants' fields spread across typed nullable columns. A given `kind` populates
-- only its own columns; the rest stay NULL. `seq` is an **application-assigned**
-- monotonic order key — `apply` sets it to max(seq)+1 within the single-writer
-- transaction (AR-1). This keeps the DDL in the portable subset (SQLite has
-- AUTOINCREMENT, Postgres uses SERIAL/IDENTITY — neither is shared, so the writer
-- owns ordering instead). Every event carries `squire` and `at`; `actor` is NULL
-- for auto/system events.
CREATE TABLE events (
    seq         BIGINT  NOT NULL PRIMARY KEY,   -- app-assigned (max(seq)+1 on append)
    kind        TEXT    NOT NULL,   -- discriminant: 'CompletionClaimed' | ... | 'RedemptionRejected'
    squire      TEXT    NOT NULL,   -- UserId; present on every variant
    actor       TEXT,               -- approving/acting Knight; NULL = auto/system
    at          BIGINT  NOT NULL,   -- Timestamp (unix millis)

    -- claim lifecycle
    claim_id    TEXT,
    quest_id    TEXT,
    on_date     BIGINT,             -- the `on: Date` of a claim

    -- approvals / adjustments / redemptions carry points-ish amounts
    points      BIGINT,             -- CompletionApproved.points / AchievementUnlocked.bonus / ItemRedeemed.cost
    amount      BIGINT,             -- PointsAdjusted.amount (signed)
    reason      TEXT,               -- rejection / adjustment reason

    -- redemption lifecycle
    request_id  TEXT,
    command_id  TEXT,
    item_id     TEXT,

    -- achievement unlock
    achievement_id TEXT
);

-- ─── CREDENTIALS (per-tenant auth secrets, SQUIRE-S-0007 / REQ-1.6) ──────────
--
-- Member secrets live INSIDE the tenant schema (never a global/shared directory),
-- so a household's credentials are isolated exactly like the rest of its data.
-- This is NOT a domain table: the `Identity` impl writes/reads it directly
-- (bypassing Repository::apply), so it carries no audit columns — just the hashed
-- secret keyed by UserId. `secret_hash` holds an Argon2id PHC string; the plaintext
-- secret is never stored. Portable subset (TEXT only), so it migrates on both
-- SQLite and Postgres.
CREATE TABLE credentials (
    user_id     TEXT NOT NULL PRIMARY KEY,  -- UserId (u128 as decimal string)
    secret_hash TEXT NOT NULL               -- Argon2id PHC string
);

-- ─── PAIRING CODES (per-tenant, ephemeral device-pairing tokens, ADR SQUIRE-A-0010) ──
--
-- A Knight mints a one-time code (via the Keep) to pair a phone to a member; the phone
-- exchanges it at POST /pair for that member's tenant-scoped token. Like `credentials`,
-- this is auth material written/read directly by the Identity layer (NOT through
-- Repository::apply, so no audit columns and not in the event log). Only the HASH of the
-- code is stored (sha256 hex) — the plaintext code lives only in the QR. Single-use is
-- enforced by deleting the row on consume; `expires_at` bounds the 30-min TTL. Portable
-- subset (TEXT + BIGINT), so it migrates on both SQLite and Postgres.
CREATE TABLE pairing_codes (
    code_hash   TEXT   NOT NULL PRIMARY KEY,  -- sha256(code) hex; plaintext never stored
    user_id     TEXT   NOT NULL,              -- target member (UserId decimal string)
    role        TEXT   NOT NULL,              -- 'Knight' | 'Squire'
    expires_at  BIGINT NOT NULL               -- unix millis; consume rejects when now >= this
);

-- Order index (primary read path is the whole ordered log).
CREATE INDEX idx_events_seq ON events (seq);
-- Per-quest / per-item raw queries.
CREATE INDEX idx_events_quest ON events (quest_id);
CREATE INDEX idx_events_item ON events (item_id);
-- Per-squire filtering (balance/streaks are derived per Squire).
CREATE INDEX idx_events_squire ON events (squire);

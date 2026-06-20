-- Real-money "cash" award per chore (SQUIRE-T-0099). Additive column, default 0 → existing quests
-- pay no cash, so no behavior changes. Whole dollars (BIGINT), portable SQLite + Postgres.
ALTER TABLE quests ADD COLUMN cash BIGINT NOT NULL DEFAULT 0;

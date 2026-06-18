-- Household configuration (ADR SQUIRE-A-0011) — a per-tenant key/value settings table.
--
-- LONG, not wide: settings grow by adding ROWS, not columns, so no migration is needed per new
-- setting. The `key` PRIMARY KEY is the lookup index (fast point reads). Portable DDL (SQLite +
-- Postgres): only TEXT / BIGINT. Audit columns per ADR A-0007 (`updated_by` = acting Knight as
-- TEXT, nullable for system/seed writes).
CREATE TABLE config (
    key        TEXT   NOT NULL PRIMARY KEY,
    value      TEXT   NOT NULL,
    updated_by TEXT,
    updated_at BIGINT NOT NULL
);

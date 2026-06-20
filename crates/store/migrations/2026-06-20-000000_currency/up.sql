-- Generalized multi-currency model (ADR SQUIRE-A-0013 / SQUIRE-T-0097).
--
-- Additive + a deterministic rename, so NO balance moves (the migration invariant): every existing
-- `PointsAdjusted` row was implicitly Coins, so it becomes `Adjusted` with currency='Coins'. The new
-- `events.currency` column is NULL for all non-adjust events (they're the Coins path by policy) and
-- 'Coins'/'Cash'/… for adjustments.
ALTER TABLE events ADD COLUMN currency TEXT;
UPDATE events SET currency = 'Coins' WHERE kind = 'PointsAdjusted';
UPDATE events SET kind = 'Adjusted' WHERE kind = 'PointsAdjusted';

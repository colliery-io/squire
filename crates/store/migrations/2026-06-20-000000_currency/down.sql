UPDATE events SET kind = 'PointsAdjusted' WHERE kind = 'Adjusted' AND (currency IS NULL OR currency = 'Coins');
ALTER TABLE events DROP COLUMN currency;

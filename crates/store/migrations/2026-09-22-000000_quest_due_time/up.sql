-- Per-chore due time (SQUIRE-T-0142): minutes since local midnight, in the household timezone.
-- Nullable and additive — existing quests have no particular time, which is the default and must
-- stay effortless. Presentation + reminders only: nothing in the domain engine reads it, so a chore
-- is never "late" and a late claim is never refused.
ALTER TABLE quests ADD COLUMN due_time INTEGER;

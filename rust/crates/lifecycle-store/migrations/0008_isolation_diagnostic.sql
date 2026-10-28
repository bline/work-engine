-- Applies to semantic schema6 and schema7. Resulting physical schema IDs are
-- 8 and 9 respectively; installing attribution does not opt into restart epochs.
ALTER TABLE native_process_exits ADD COLUMN isolation_diagnostic TEXT;

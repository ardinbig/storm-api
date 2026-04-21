-- 002: Link agent accounts to a station (system user)
--
-- Each agent may optionally belong to a station, represented by a row in the
-- `users` table.  The column is nullable so that existing agents and the
-- special house account are unaffected.
--
-- Cascade rules:
--   ON DELETE SET NULL  — deleting a station user clears the FK (agent keeps
--                         its account but loses the station reference).
--   ON UPDATE CASCADE   — propagates primary-key changes automatically.

ALTER TABLE agent_accounts
    ADD COLUMN IF NOT EXISTS station_id UUID DEFAULT NULL,
    ADD CONSTRAINT fk_agent_accounts_station
        FOREIGN KEY (station_id)
        REFERENCES users(id)
        ON DELETE SET NULL
        ON UPDATE CASCADE;

CREATE INDEX IF NOT EXISTS idx_agent_accounts_station_id
    ON agent_accounts(station_id);


-- 004: Seed and protect the house commission account
--
-- The withdrawal flow credits commissions into a special agent account with
-- agent_ref = 'STORM-ACCOUNT-0000'. This migration ensures the row exists and
-- cannot be deleted directly from the database.

INSERT INTO agent_accounts (id, agent_ref, name, password, balance, currency_code)
VALUES (gen_random_uuid(), 'STORM-ACCOUNT-0000', 'House Account', NULL, 0, 'CDF')
ON CONFLICT (agent_ref) DO NOTHING;

CREATE OR REPLACE FUNCTION fn_protect_house_account()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.agent_ref = 'STORM-ACCOUNT-0000' THEN
        RAISE EXCEPTION
            'The house commission account is protected and cannot be deleted.';
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER trg_protect_house_account
    BEFORE DELETE ON agent_accounts
    FOR EACH ROW
    EXECUTE FUNCTION fn_protect_house_account();

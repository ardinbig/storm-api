-- Storm PostgreSQL Schema — Consolidated Migration
-- Fuel station management system
-- All types, constraints, triggers, views, and indexes in a single file.


-- UTILITY FUNCTIONS

-- Auto-update trigger function for updated_at columns
CREATE OR REPLACE FUNCTION update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 1. TABLES (ordered by FK dependencies)

-- 1.1 Card master table
CREATE TABLE IF NOT EXISTS cards (
    id UUID PRIMARY KEY,
    card_id VARCHAR(255) NOT NULL UNIQUE,
    status VARCHAR(45) DEFAULT 'Production'
);

-- 1.2 Vehicle/customer categories
CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    name VARCHAR(30) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO categories (id, name) VALUES
    ('a0000000-0000-0000-0000-000000000001', 'Motorbike'),
    ('a0000000-0000-0000-0000-000000000002', 'Bus')
ON CONFLICT (id) DO NOTHING;

-- 1.3 System users (station/admin login)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT DEFAULT NULL,
    password TEXT NOT NULL,
    username TEXT NOT NULL UNIQUE
);

-- 1.4 Commission rates
CREATE TABLE IF NOT EXISTS commissions (
    id UUID PRIMARY KEY,
    percentage DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1.5 Commission tiers (MLM loyalty levels)
CREATE TABLE IF NOT EXISTS commission_tiers (
    id UUID PRIMARY KEY,
    level1 DOUBLE PRECISION NOT NULL,
    level2 DOUBLE PRECISION NOT NULL,
    category VARCHAR(255) DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1.6 Fuel prices
CREATE TABLE IF NOT EXISTS prices (
    id UUID PRIMARY KEY,
    consumption_type VARCHAR(30) NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    price_date TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1.7 Agent accounts
CREATE TABLE IF NOT EXISTS agent_accounts (
    id UUID PRIMARY KEY,
    agent_ref VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(45) DEFAULT NULL,
    password VARCHAR(255) DEFAULT NULL,
    balance DOUBLE PRECISION DEFAULT 0,
    currency_code VARCHAR(255) NOT NULL DEFAULT 'CDF',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trg_agent_accounts_updated_at
    BEFORE UPDATE ON agent_accounts
    FOR EACH ROW EXECUTE FUNCTION update_timestamp();

-- 1.8 Card details (balances & credentials)
CREATE TABLE IF NOT EXISTS card_details (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    amount DOUBLE PRECISION NOT NULL DEFAULT 0,
    nfc_ref VARCHAR(255) NOT NULL UNIQUE,
    registration_code VARCHAR(30) NOT NULL UNIQUE,
    password VARCHAR(255) DEFAULT NULL,
    network VARCHAR(255) DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_card_details_cards FOREIGN KEY (nfc_ref) REFERENCES cards(card_id)
);

CREATE TRIGGER trg_card_details_updated_at
    BEFORE UPDATE ON card_details
    FOR EACH ROW EXECUTE FUNCTION update_timestamp();

-- 1.9 Customer profiles
CREATE TABLE IF NOT EXISTS customers (
    id UUID PRIMARY KEY,
    client_code VARCHAR(255) DEFAULT NULL UNIQUE,
    name VARCHAR(255) DEFAULT NULL,
    first_name VARCHAR(255) DEFAULT NULL,
    last_name VARCHAR(255) DEFAULT NULL,
    address VARCHAR(255) DEFAULT NULL,
    networks VARCHAR(255) DEFAULT NULL,
    phone VARCHAR(15) DEFAULT NULL,
    category_ref UUID DEFAULT NULL,
    card_id VARCHAR(255) DEFAULT NULL,
    gender VARCHAR(15) DEFAULT NULL,
    marital_status VARCHAR(25) DEFAULT NULL,
    affiliation VARCHAR(25) DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_customers_cards FOREIGN KEY (card_id) REFERENCES cards(card_id),
    CONSTRAINT fk_customers_categories FOREIGN KEY (category_ref) REFERENCES categories(id)
);

CREATE TRIGGER trg_customers_updated_at
    BEFORE UPDATE ON customers
    FOR EACH ROW EXECUTE FUNCTION update_timestamp();

-- 1.10 Customer enrollment / registrations
CREATE TABLE IF NOT EXISTS registrations (
    id UUID PRIMARY KEY,
    card_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) DEFAULT NULL,
    last_name VARCHAR(255) DEFAULT NULL,
    first_name VARCHAR(255) DEFAULT NULL,
    gender VARCHAR(15) DEFAULT NULL,
    affiliation VARCHAR(25) DEFAULT NULL,
    phone VARCHAR(15) DEFAULT NULL,
    address VARCHAR(255) DEFAULT NULL,
    network VARCHAR(255) DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status INTEGER NOT NULL DEFAULT 1,
    password VARCHAR(255) DEFAULT NULL,
    category UUID DEFAULT NULL,
    state INTEGER NOT NULL DEFAULT 1
);

-- 1.11 Fuel consumption log
CREATE TABLE IF NOT EXISTS consumptions (
    id UUID PRIMARY KEY,
    client_ref VARCHAR(255) NOT NULL,
    consumption_type VARCHAR(30) NOT NULL,
    quantity DOUBLE PRECISION NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    username VARCHAR(255) NOT NULL,
    consumption_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status INTEGER NOT NULL DEFAULT 1,
    CONSTRAINT fk_consumptions_customers FOREIGN KEY (client_ref) REFERENCES customers(client_code)
);

-- 1.12 Financial transactions
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    date TIMESTAMPTZ DEFAULT NOW(),
    transaction_type VARCHAR(255) DEFAULT NULL,
    client_account VARCHAR(255) DEFAULT NULL,
    agent_account VARCHAR(255) DEFAULT NULL,
    amount DOUBLE PRECISION DEFAULT 0,
    currency_code VARCHAR(10) DEFAULT NULL,
    commission DOUBLE PRECISION DEFAULT 0,
    CONSTRAINT fk_transactions_agents FOREIGN KEY (agent_account) REFERENCES agent_accounts(agent_ref)
);

-- 1.13 Loyalty bonuses
CREATE TABLE IF NOT EXISTS bonuses (
    id UUID PRIMARY KEY,
    client_ref VARCHAR(255) NOT NULL,
    quantity NUMERIC(15, 2) NOT NULL,
    price NUMERIC(15, 2) NOT NULL,
    percentage NUMERIC(15, 2) NOT NULL,
    amount_local NUMERIC(15, 2) NOT NULL,
    amount_foreign NUMERIC(15, 2) NOT NULL,
    networks VARCHAR(255) NOT NULL,
    networks_alt VARCHAR(255) DEFAULT NULL,
    percentage_alt NUMERIC(15, 2) DEFAULT NULL,
    phone VARCHAR(15) NOT NULL,
    phone_alt VARCHAR(15) DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1.14 Deletion audit log
CREATE TABLE IF NOT EXISTS deleted_records (
    id UUID PRIMARY KEY,
    agent_ref VARCHAR(255) DEFAULT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    client_ref VARCHAR(255) DEFAULT NULL,
    deleted_quantity DOUBLE PRECISION NOT NULL,
    consumption_type VARCHAR(255) NOT NULL
);

-- 2. VIEWS

-- 2.1 Regular consumers summary
CREATE OR REPLACE VIEW regular_consumers AS
SELECT
    COALESCE(SUM(co.quantity), 0)::NUMERIC(32, 0) AS total_quantity,
    cu.name,
    cu.client_code,
    cu.phone
FROM customers cu
LEFT JOIN consumptions co ON co.client_ref = cu.client_code
GROUP BY cu.name, cu.client_code, cu.phone
ORDER BY total_quantity DESC;

-- 2.2 Best customers for SMS
CREATE OR REPLACE VIEW best_customers_sms AS
SELECT
    cu.client_code AS client_ref,
    cu.name,
    cu.phone,
    COALESCE(SUM(co.quantity), 0)::INTEGER AS quantity
FROM customers cu
LEFT JOIN consumptions co ON co.client_ref = cu.client_code
GROUP BY cu.client_code, cu.name, cu.phone
ORDER BY quantity DESC;

-- 2.3 Withdrawal summary
CREATE OR REPLACE VIEW withdrawal_summary AS
SELECT
    t.date            AS transaction_date,
    t.transaction_type AS movement_type,
    t.client_account  AS client_card,
    c.name            AS beneficiary,
    c.phone           AS phone,
    t.agent_account   AS agent_ref,
    a.name            AS agent_name,
    t.amount          AS amount,
    a.currency_code   AS currency
FROM transactions t
    INNER JOIN customers c  ON t.client_account = c.card_id
    INNER JOIN agent_accounts a ON t.agent_account = a.agent_ref;

-- 2.4 Full withdrawal summary with commission
CREATE OR REPLACE VIEW withdrawal_summary_full AS
SELECT
    t.date            AS transaction_date,
    t.transaction_type AS movement_type,
    t.client_account  AS client_card,
    c.name            AS beneficiary,
    c.phone           AS phone,
    t.agent_account   AS agent_ref,
    a.name            AS agent_name,
    t.amount          AS amount,
    t.commission      AS commission,
    a.currency_code   AS currency
FROM transactions t
    INNER JOIN customers c  ON t.client_account = c.card_id
    INNER JOIN agent_accounts a ON t.agent_account = a.agent_ref;

-- 3. BUSINESS TRIGGERS

-- 3.1 Auto-update card status to 'Production' when card_details is inserted
CREATE OR REPLACE FUNCTION tg_change_card_status()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE cards SET status = 'Production' WHERE card_id = NEW.nfc_ref;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER tg_change_card_status_trigger
    AFTER INSERT ON card_details
    FOR EACH ROW
    EXECUTE FUNCTION tg_change_card_status();

-- 3.2 Auto-create card_details when a customer is inserted
--     (only if a registration with a hashed password exists for that card)
CREATE OR REPLACE FUNCTION fn_customer_insert_card_details()
RETURNS TRIGGER AS $$
DECLARE
    reg_password VARCHAR(255);
BEGIN
    IF NEW.card_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT r.password INTO reg_password
    FROM registrations r
    WHERE r.card_id = NEW.card_id
    LIMIT 1;

    IF FOUND AND reg_password IS NOT NULL THEN
        INSERT INTO card_details (nfc_ref, registration_code, password, network)
        VALUES (NEW.card_id, NEW.client_code, reg_password, NEW.networks)
        ON CONFLICT (nfc_ref) DO NOTHING;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_customer_insert_card_details
    AFTER INSERT ON customers
    FOR EACH ROW
    EXECUTE FUNCTION fn_customer_insert_card_details();

-- 3.3 Sync card_details when a customer is updated
CREATE OR REPLACE FUNCTION fn_customer_update_card_details()
RETURNS TRIGGER AS $$
DECLARE
    existing_detail_id UUID;
BEGIN
    IF NEW.card_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT cd.id INTO existing_detail_id
    FROM card_details cd
    WHERE cd.registration_code = NEW.client_code
    LIMIT 1;

    IF FOUND THEN
        UPDATE card_details
        SET nfc_ref = NEW.card_id
        WHERE id = existing_detail_id;
    ELSE
        -- Insert with NULL password; app layer must set it before use
        INSERT INTO card_details (nfc_ref, registration_code, password, network)
        VALUES (NEW.card_id, NEW.client_code, NULL, NEW.networks)
        ON CONFLICT (nfc_ref) DO NOTHING;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_customer_update_card_details
    AFTER UPDATE ON customers
    FOR EACH ROW
    EXECUTE FUNCTION fn_customer_update_card_details();

-- 3.4 MLM 2-level loyalty bonus on consumption insert
CREATE OR REPLACE FUNCTION fn_consumption_bonus_tree()
RETURNS TRIGGER AS $$
DECLARE
    tier_level1 DOUBLE PRECISION;
    tier_level2 DOUBLE PRECISION;
    sponsor_code VARCHAR(255);
BEGIN
    SELECT ct.level1, ct.level2
    INTO tier_level1, tier_level2
    FROM commission_tiers ct
    WHERE ct.category = NEW.consumption_type
    LIMIT 1;

    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    -- Level 1: credit consuming customer's card balance
    UPDATE card_details
    SET amount = amount + (NEW.quantity * tier_level1)
    WHERE registration_code = NEW.client_ref;

    -- Level 2: credit sponsor's card balance
    SELECT cd.network INTO sponsor_code
    FROM card_details cd
    WHERE cd.registration_code = NEW.client_ref
    LIMIT 1;

    IF sponsor_code IS NOT NULL AND sponsor_code <> '' THEN
        UPDATE card_details
        SET amount = amount + (NEW.quantity * tier_level2)
        WHERE registration_code = sponsor_code;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_consumption_bonus_tree
    AFTER INSERT ON consumptions
    FOR EACH ROW
    EXECUTE FUNCTION fn_consumption_bonus_tree();

-- 4. INDEXES

CREATE INDEX IF NOT EXISTS idx_customers_card_id ON customers(card_id);
CREATE INDEX IF NOT EXISTS idx_customers_phone ON customers(phone);
CREATE INDEX IF NOT EXISTS idx_transactions_client_account ON transactions(client_account);
CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date DESC);
CREATE INDEX IF NOT EXISTS idx_consumptions_client_ref ON consumptions(client_ref);
CREATE INDEX IF NOT EXISTS idx_consumptions_date ON consumptions(consumption_date DESC);
CREATE INDEX IF NOT EXISTS idx_transactions_agent_account ON transactions(agent_account);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

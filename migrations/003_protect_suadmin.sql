-- 003: Protect the suadmin account from deletion
--
-- A BEFORE DELETE trigger on `users` raises an exception when any statement
-- tries to remove the row whose username is 'suadmin'.  The check lives in
-- the database so it cannot be bypassed by the application layer.

CREATE OR REPLACE FUNCTION fn_protect_suadmin()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.username = 'suadmin' THEN
        RAISE EXCEPTION
            'The suadmin account is protected and cannot be deleted.';
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER trg_protect_suadmin
    BEFORE DELETE ON users
    FOR EACH ROW
    EXECUTE FUNCTION fn_protect_suadmin();

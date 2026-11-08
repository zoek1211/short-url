CREATE FUNCTION links_reject_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'links are immutable: % is not allowed', TG_OP
        USING ERRCODE = 'restrict_violation';
END;
$$;

CREATE TRIGGER links_no_update_delete
    BEFORE UPDATE OR DELETE ON links
    FOR EACH ROW EXECUTE FUNCTION links_reject_change();

CREATE TRIGGER links_no_truncate
    BEFORE TRUNCATE ON links
    FOR EACH STATEMENT EXECUTE FUNCTION links_reject_change();

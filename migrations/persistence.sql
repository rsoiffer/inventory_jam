CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS player_state (
    username TEXT PRIMARY KEY,
    state JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS player_transaction (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL REFERENCES player_state (username),
    kind TEXT NOT NULL,
    payload JSONB NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS player_transaction_username_applied_at_idx
    ON player_transaction (username, applied_at);

CREATE OR REPLACE FUNCTION player_state_hash(p_username TEXT, p_state JSONB)
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
STRICT
SECURITY DEFINER
SET search_path = public
AS $$
    SELECT encode(
        digest(
            convert_to(
                'v1|'
                    || p_username
                    || '|fragment_count='
                    || (p_state->>'fragment_count'),
                'UTF8'
            ),
            'sha256'
        ),
        'hex'
    );
$$;

CREATE OR REPLACE FUNCTION ensure_player(p_username TEXT)
RETURNS JSONB
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    current_state JSONB;
BEGIN
    SELECT state INTO current_state
    FROM player_state
    WHERE username = p_username;

    IF FOUND THEN
        RETURN current_state;
    END IF;

    current_state := jsonb_build_object(
        'version', 1,
        'fragment_count', 0
    );

    INSERT INTO player_state (username, state)
    VALUES (p_username, current_state)
    ON CONFLICT (username) DO NOTHING;

    SELECT state INTO current_state
    FROM player_state
    WHERE username = p_username;

    RETURN current_state;
END;
$$;

CREATE OR REPLACE FUNCTION apply_player_transaction(
    p_id UUID,
    p_username TEXT,
    p_kind TEXT,
    p_payload JSONB
)
RETURNS JSONB
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    current_state JSONB;
    amount BIGINT;
    next_count BIGINT;
BEGIN
    IF EXISTS (SELECT 1 FROM player_transaction WHERE id = p_id) THEN
        SELECT state INTO current_state
        FROM player_state
        WHERE username = p_username
        FOR UPDATE;

        IF NOT FOUND THEN
            RAISE EXCEPTION 'player % missing for idempotent transaction %', p_username, p_id;
        END IF;

        RETURN current_state;
    END IF;

    PERFORM ensure_player(p_username);

    SELECT state INTO current_state
    FROM player_state
    WHERE username = p_username
    FOR UPDATE;

    IF p_kind = 'add_fragments' THEN
        IF jsonb_typeof(p_payload->'amount') <> 'number' THEN
            RAISE EXCEPTION 'add_fragments payload.amount must be a number';
        END IF;

        amount := (p_payload->>'amount')::BIGINT;
        IF amount IS NULL OR amount <= 0 OR amount > 4294967295 THEN
            RAISE EXCEPTION 'add_fragments amount out of range: %', amount;
        END IF;

        next_count := (current_state->>'fragment_count')::BIGINT + amount;
        IF next_count > 4294967295 THEN
            RAISE EXCEPTION 'fragment_count overflow: %', next_count;
        END IF;

        current_state := jsonb_set(
            current_state,
            '{fragment_count}',
            to_jsonb(next_count::INT8),
            false
        );
    ELSE
        RAISE EXCEPTION 'unknown transaction kind: %', p_kind;
    END IF;

    UPDATE player_state
    SET state = current_state,
        updated_at = now()
    WHERE username = p_username;

    INSERT INTO player_transaction (id, username, kind, payload)
    VALUES (p_id, p_username, p_kind, p_payload);

    RETURN current_state;
END;
$$;

CREATE OR REPLACE FUNCTION get_player_states(p_usernames TEXT[])
RETURNS TABLE (username TEXT, state JSONB)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    PERFORM ensure_player(u)
    FROM unnest(COALESCE(p_usernames, ARRAY[]::TEXT[])) AS u;

    RETURN QUERY
    SELECT ps.username, ps.state
    FROM player_state AS ps
    WHERE ps.username = ANY (p_usernames);
END;
$$;

CREATE OR REPLACE FUNCTION flush_player_transactions(
    p_transactions JSONB,
    p_usernames TEXT[]
)
RETURNS TABLE (username TEXT, state_hash TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    tx JSONB;
BEGIN
    IF jsonb_typeof(p_transactions) <> 'array' THEN
        RAISE EXCEPTION 'transactions must be a JSON array';
    END IF;

    FOR tx IN SELECT value FROM jsonb_array_elements(p_transactions) AS value
    LOOP
        PERFORM apply_player_transaction(
            (tx->>'id')::UUID,
            tx->>'username',
            tx->>'kind',
            COALESCE(tx->'payload', '{}'::JSONB)
        );
    END LOOP;

    PERFORM ensure_player(u)
    FROM unnest(COALESCE(p_usernames, ARRAY[]::TEXT[])) AS u;

    RETURN QUERY
    SELECT ps.username, player_state_hash(ps.username, ps.state)
    FROM player_state AS ps
    WHERE ps.username = ANY (p_usernames);
END;
$$;

-- Minimal privileges: app roles cannot touch tables or internal helpers directly.
REVOKE ALL ON TABLE player_state FROM PUBLIC;
REVOKE ALL ON TABLE player_transaction FROM PUBLIC;
REVOKE ALL ON TABLE player_state FROM inventory_jam_server;
REVOKE ALL ON TABLE player_transaction FROM inventory_jam_server;

REVOKE ALL ON FUNCTION player_state_hash(TEXT, JSONB) FROM PUBLIC;
REVOKE ALL ON FUNCTION ensure_player(TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION apply_player_transaction(UUID, TEXT, TEXT, JSONB) FROM PUBLIC;
REVOKE ALL ON FUNCTION get_player_states(TEXT[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION flush_player_transactions(JSONB, TEXT[]) FROM PUBLIC;

REVOKE ALL ON FUNCTION player_state_hash(TEXT, JSONB) FROM inventory_jam_server;
REVOKE ALL ON FUNCTION ensure_player(TEXT) FROM inventory_jam_server;
REVOKE ALL ON FUNCTION apply_player_transaction(UUID, TEXT, TEXT, JSONB) FROM inventory_jam_server;

-- Server may only call the two batch API functions.
GRANT EXECUTE ON FUNCTION get_player_states(TEXT[]) TO inventory_jam_server;
GRANT EXECUTE ON FUNCTION flush_player_transactions(JSONB, TEXT[]) TO inventory_jam_server;

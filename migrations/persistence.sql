CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS player_state (
    username TEXT PRIMARY KEY,
    state JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS player_transaction (
    id UUID PRIMARY KEY,
    changes JSONB NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS player_transaction_applied_at_idx
    ON player_transaction (applied_at);

-- Convert legacy single-balance player rows to per-weapon progression state.
UPDATE player_state
SET state = jsonb_build_object('version', 2, 'weapons', '{}'::JSONB),
    updated_at = now()
WHERE COALESCE(state->>'version', '1') <> '2'
   OR NOT (state ? 'weapons');

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
                'v2|'
                    || p_username
                    || '|weapons=['
                    || COALESCE(
                        (
                            SELECT string_agg(
                                key
                                    || ':{fragments='
                                    || COALESCE(value->>'fragments', '0')
                                    || ',level='
                                    || COALESCE(value->>'level', '0')
                                    || '}',
                                ','
                                ORDER BY key
                            )
                            FROM jsonb_each(COALESCE(p_state->'weapons', '{}'::JSONB))
                        ),
                        ''
                    )
                    || ']',
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
        'version', 2,
        'weapons', '{}'::JSONB
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

CREATE OR REPLACE FUNCTION apply_player_change(
    p_username TEXT,
    p_weapon_id INT,
    p_fragment_delta BIGINT,
    p_level_delta BIGINT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    current_state JSONB;
    weapon_key TEXT;
    weapon_state JSONB;
    current_fragments BIGINT;
    current_level BIGINT;
    next_fragments BIGINT;
    next_level BIGINT;
BEGIN
    PERFORM ensure_player(p_username);

    SELECT state INTO current_state
    FROM player_state
    WHERE username = p_username
    FOR UPDATE;

    IF p_weapon_id IS NULL OR p_weapon_id < 0 OR p_weapon_id > 65535 THEN
        RAISE EXCEPTION 'weapon_id out of range: %', p_weapon_id;
    END IF;

    IF p_fragment_delta IS NULL OR p_level_delta IS NULL THEN
        RAISE EXCEPTION 'fragment_delta and level_delta are required';
    END IF;

    IF p_fragment_delta = 0 AND p_level_delta = 0 THEN
        RAISE EXCEPTION 'fragment_delta and level_delta cannot both be zero';
    END IF;

    weapon_key := p_weapon_id::TEXT;
    weapon_state := COALESCE(current_state->'weapons'->weapon_key, '{"fragments":0,"level":0}'::JSONB);
    current_fragments := COALESCE((weapon_state->>'fragments')::BIGINT, 0);
    current_level := COALESCE((weapon_state->>'level')::BIGINT, 0);

    next_fragments := current_fragments + p_fragment_delta;
    IF next_fragments < 0 THEN
        RAISE EXCEPTION 'fragment balance cannot go negative: % + %', current_fragments, p_fragment_delta;
    END IF;
    IF next_fragments > 4294967295 THEN
        RAISE EXCEPTION 'fragment balance overflow: %', next_fragments;
    END IF;

    next_level := current_level + p_level_delta;
    IF next_level < 0 THEN
        RAISE EXCEPTION 'level cannot go negative: % + %', current_level, p_level_delta;
    END IF;
    IF next_level > 4294967295 THEN
        RAISE EXCEPTION 'level overflow: %', next_level;
    END IF;

    weapon_state := jsonb_build_object(
        'fragments', next_fragments,
        'level', next_level
    );

    current_state := jsonb_set(
        COALESCE(current_state, '{"version":2,"weapons":{}}'::JSONB),
        ARRAY['weapons', weapon_key],
        weapon_state,
        true
    );
    current_state := jsonb_set(current_state, '{version}', '2'::JSONB, true);

    UPDATE player_state
    SET state = current_state,
        updated_at = now()
    WHERE username = p_username;
END;
$$;

-- Applies every player change in one atomic transaction. Idempotent on p_id.
CREATE OR REPLACE FUNCTION apply_persistence_transaction(
    p_id UUID,
    p_changes JSONB
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    change JSONB;
BEGIN
    IF EXISTS (SELECT 1 FROM player_transaction WHERE id = p_id) THEN
        RETURN;
    END IF;

    IF jsonb_typeof(p_changes) <> 'array' OR jsonb_array_length(p_changes) = 0 THEN
        RAISE EXCEPTION 'transaction changes must be a non-empty JSON array';
    END IF;

    FOR change IN SELECT value FROM jsonb_array_elements(p_changes) AS value
    LOOP
        PERFORM apply_player_change(
            change->>'username',
            (change->>'weapon_id')::INT,
            (change->>'fragment_delta')::BIGINT,
            (change->>'level_delta')::BIGINT
        );
    END LOOP;

    INSERT INTO player_transaction (id, changes)
    VALUES (p_id, p_changes);
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

DROP FUNCTION IF EXISTS flush_player_transactions(JSONB, TEXT[]);

CREATE FUNCTION flush_player_transactions(
    p_transactions JSONB,
    p_usernames TEXT[]
)
RETURNS TABLE (
    transaction_id UUID,
    applied BOOLEAN,
    error_message TEXT,
    username TEXT,
    state_hash TEXT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    tx JSONB;
    current_transaction_id UUID;
    player_record RECORD;
BEGIN
    IF jsonb_typeof(p_transactions) <> 'array' THEN
        RAISE EXCEPTION 'transactions must be a JSON array';
    END IF;

    FOR tx IN SELECT value FROM jsonb_array_elements(p_transactions) AS value
    LOOP
        current_transaction_id := NULL;
        BEGIN
            current_transaction_id := (tx->>'id')::UUID;
            PERFORM apply_persistence_transaction(
                current_transaction_id,
                COALESCE(tx->'changes', '[]'::JSONB)
            );

            transaction_id := current_transaction_id;
            applied := true;
            error_message := NULL;
            username := NULL;
            state_hash := NULL;
            RETURN NEXT;
        EXCEPTION WHEN OTHERS THEN
            transaction_id := current_transaction_id;
            applied := false;
            error_message := SQLERRM;
            username := NULL;
            state_hash := NULL;
            RETURN NEXT;
        END;
    END LOOP;

    PERFORM ensure_player(u)
    FROM unnest(COALESCE(p_usernames, ARRAY[]::TEXT[])) AS u;

    FOR player_record IN
        SELECT ps.username, player_state_hash(ps.username, ps.state) AS state_hash
        FROM player_state AS ps
        WHERE ps.username = ANY (p_usernames)
        ORDER BY ps.username
    LOOP
        transaction_id := NULL;
        applied := NULL;
        error_message := NULL;
        username := player_record.username;
        state_hash := player_record.state_hash;
        RETURN NEXT;
    END LOOP;
END;
$$;

-- Minimal privileges: app roles cannot touch tables or internal helpers directly.
REVOKE ALL ON TABLE player_state FROM PUBLIC;
REVOKE ALL ON TABLE player_transaction FROM PUBLIC;
REVOKE ALL ON TABLE player_state FROM inventory_jam_server;
REVOKE ALL ON TABLE player_transaction FROM inventory_jam_server;

REVOKE ALL ON FUNCTION player_state_hash(TEXT, JSONB) FROM PUBLIC;
REVOKE ALL ON FUNCTION ensure_player(TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION apply_player_change(TEXT, INT, BIGINT, BIGINT) FROM PUBLIC;
REVOKE ALL ON FUNCTION apply_persistence_transaction(UUID, JSONB) FROM PUBLIC;
REVOKE ALL ON FUNCTION get_player_states(TEXT[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION flush_player_transactions(JSONB, TEXT[]) FROM PUBLIC;

REVOKE ALL ON FUNCTION player_state_hash(TEXT, JSONB) FROM inventory_jam_server;
REVOKE ALL ON FUNCTION ensure_player(TEXT) FROM inventory_jam_server;
REVOKE ALL ON FUNCTION apply_player_change(TEXT, INT, BIGINT, BIGINT) FROM inventory_jam_server;
REVOKE ALL ON FUNCTION apply_persistence_transaction(UUID, JSONB) FROM inventory_jam_server;

-- Drop the old apply_player_change(TEXT, BIGINT) if it still exists.
DROP FUNCTION IF EXISTS apply_player_change(TEXT, BIGINT);

-- Server may only call the two batch API functions.
GRANT EXECUTE ON FUNCTION get_player_states(TEXT[]) TO inventory_jam_server;
GRANT EXECUTE ON FUNCTION flush_player_transactions(JSONB, TEXT[]) TO inventory_jam_server;

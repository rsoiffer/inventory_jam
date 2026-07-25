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

-- Convert legacy player rows to versioned per-weapon progression state (v6).
UPDATE player_state
SET state = jsonb_build_object(
        'version', 6,
        'equipped_weapon_id', COALESCE((state->>'equipped_weapon_id')::INT, 0),
        'weapons', COALESCE(
            (
                SELECT jsonb_object_agg(
                    key,
                    jsonb_build_object(
                        'fragments', COALESCE((value->>'fragments')::BIGINT, 0),
                        'damage_level', COALESCE(
                            (value->>'damage_level')::BIGINT,
                            (value->>'level')::BIGINT,
                            0
                        ),
                        'attack_speed_level', COALESCE((value->>'attack_speed_level')::BIGINT, 0),
                        'pierce_level', COALESCE((value->>'pierce_level')::BIGINT, 0),
                        'crit_level', COALESCE((value->>'crit_level')::BIGINT, 0),
                        'armor_level', COALESCE((value->>'armor_level')::BIGINT, 0),
                        'max_health_level', COALESCE((value->>'max_health_level')::BIGINT, 0)
                    )
                )
                FROM jsonb_each(COALESCE(state->'weapons', '{}'::JSONB))
            ),
            '{}'::JSONB
        )
    ),
    updated_at = now()
WHERE COALESCE(state->>'version', '1') <> '6'
   OR NOT (state ? 'weapons')
   OR NOT (state ? 'equipped_weapon_id');

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
                'v6|'
                    || p_username
                    || '|equipped='
                    || COALESCE(p_state->>'equipped_weapon_id', '0')
                    || '|weapons=['
                    || COALESCE(
                        (
                            SELECT string_agg(
                                key
                                    || ':{fragments='
                                    || COALESCE(value->>'fragments', '0')
                                    || ',damage_level='
                                    || COALESCE(value->>'damage_level', value->>'level', '0')
                                    || ',attack_speed_level='
                                    || COALESCE(value->>'attack_speed_level', '0')
                                    || ',pierce_level='
                                    || COALESCE(value->>'pierce_level', '0')
                                    || ',crit_level='
                                    || COALESCE(value->>'crit_level', '0')
                                    || ',armor_level='
                                    || COALESCE(value->>'armor_level', '0')
                                    || ',max_health_level='
                                    || COALESCE(value->>'max_health_level', '0')
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
        'version', 6,
        'equipped_weapon_id', 0,
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

DROP FUNCTION IF EXISTS apply_player_change(TEXT, INT, BIGINT, BIGINT);
DROP FUNCTION IF EXISTS apply_player_change(TEXT, BIGINT);
DROP FUNCTION IF EXISTS apply_player_change(TEXT, INT, BIGINT, BIGINT, BOOLEAN);
DROP FUNCTION IF EXISTS apply_player_change(TEXT, INT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BOOLEAN);
DROP FUNCTION IF EXISTS apply_player_change(TEXT, INT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BOOLEAN);

CREATE OR REPLACE FUNCTION apply_player_change(
    p_username TEXT,
    p_weapon_id INT,
    p_fragment_delta BIGINT,
    p_damage_level_delta BIGINT,
    p_attack_speed_level_delta BIGINT,
    p_pierce_level_delta BIGINT,
    p_crit_level_delta BIGINT,
    p_armor_level_delta BIGINT,
    p_max_health_level_delta BIGINT,
    p_set_equipped BOOLEAN
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
    current_damage_level BIGINT;
    current_attack_speed_level BIGINT;
    current_pierce_level BIGINT;
    current_crit_level BIGINT;
    current_armor_level BIGINT;
    current_max_health_level BIGINT;
    next_fragments BIGINT;
    next_damage_level BIGINT;
    next_attack_speed_level BIGINT;
    next_pierce_level BIGINT;
    next_crit_level BIGINT;
    next_armor_level BIGINT;
    next_max_health_level BIGINT;
BEGIN
    PERFORM ensure_player(p_username);

    SELECT state INTO current_state
    FROM player_state
    WHERE username = p_username
    FOR UPDATE;

    IF p_weapon_id IS NULL OR p_weapon_id < 0 OR p_weapon_id > 65535 THEN
        RAISE EXCEPTION 'weapon_id out of range: %', p_weapon_id;
    END IF;

    IF p_fragment_delta IS NULL
        OR p_damage_level_delta IS NULL
        OR p_attack_speed_level_delta IS NULL
        OR p_pierce_level_delta IS NULL
        OR p_crit_level_delta IS NULL
        OR p_armor_level_delta IS NULL
        OR p_max_health_level_delta IS NULL
        OR p_set_equipped IS NULL
    THEN
        RAISE EXCEPTION 'fragment/stat deltas and set_equipped are required';
    END IF;

    IF p_fragment_delta = 0
        AND p_damage_level_delta = 0
        AND p_attack_speed_level_delta = 0
        AND p_pierce_level_delta = 0
        AND p_crit_level_delta = 0
        AND p_armor_level_delta = 0
        AND p_max_health_level_delta = 0
        AND NOT p_set_equipped
    THEN
        RAISE EXCEPTION 'fragment/stat deltas cannot all be zero unless set_equipped';
    END IF;

    IF p_fragment_delta <> 0
        OR p_damage_level_delta <> 0
        OR p_attack_speed_level_delta <> 0
        OR p_pierce_level_delta <> 0
        OR p_crit_level_delta <> 0
        OR p_armor_level_delta <> 0
        OR p_max_health_level_delta <> 0
    THEN
        weapon_key := p_weapon_id::TEXT;
        weapon_state := COALESCE(
            current_state->'weapons'->weapon_key,
            '{"fragments":0,"damage_level":0,"attack_speed_level":0,"pierce_level":0,"crit_level":0,"armor_level":0,"max_health_level":0}'::JSONB
        );
        current_fragments := COALESCE((weapon_state->>'fragments')::BIGINT, 0);
        current_damage_level := COALESCE(
            (weapon_state->>'damage_level')::BIGINT,
            (weapon_state->>'level')::BIGINT,
            0
        );
        current_attack_speed_level := COALESCE((weapon_state->>'attack_speed_level')::BIGINT, 0);
        current_pierce_level := COALESCE((weapon_state->>'pierce_level')::BIGINT, 0);
        current_crit_level := COALESCE((weapon_state->>'crit_level')::BIGINT, 0);
        current_armor_level := COALESCE((weapon_state->>'armor_level')::BIGINT, 0);
        current_max_health_level := COALESCE((weapon_state->>'max_health_level')::BIGINT, 0);

        next_fragments := current_fragments + p_fragment_delta;
        IF next_fragments < 0 THEN
            RAISE EXCEPTION 'fragment balance cannot go negative: % + %', current_fragments, p_fragment_delta;
        END IF;
        IF next_fragments > 4294967295 THEN
            RAISE EXCEPTION 'fragment balance overflow: %', next_fragments;
        END IF;

        next_damage_level := current_damage_level + p_damage_level_delta;
        IF next_damage_level < 0 THEN
            RAISE EXCEPTION 'damage_level cannot go negative: % + %', current_damage_level, p_damage_level_delta;
        END IF;
        IF next_damage_level > 4294967295 THEN
            RAISE EXCEPTION 'damage_level overflow: %', next_damage_level;
        END IF;

        next_attack_speed_level := current_attack_speed_level + p_attack_speed_level_delta;
        IF next_attack_speed_level < 0 THEN
            RAISE EXCEPTION 'attack_speed_level cannot go negative: % + %', current_attack_speed_level, p_attack_speed_level_delta;
        END IF;
        IF next_attack_speed_level > 4294967295 THEN
            RAISE EXCEPTION 'attack_speed_level overflow: %', next_attack_speed_level;
        END IF;

        next_pierce_level := current_pierce_level + p_pierce_level_delta;
        IF next_pierce_level < 0 THEN
            RAISE EXCEPTION 'pierce_level cannot go negative: % + %', current_pierce_level, p_pierce_level_delta;
        END IF;
        IF next_pierce_level > 4294967295 THEN
            RAISE EXCEPTION 'pierce_level overflow: %', next_pierce_level;
        END IF;

        next_crit_level := current_crit_level + p_crit_level_delta;
        IF next_crit_level < 0 THEN
            RAISE EXCEPTION 'crit_level cannot go negative: % + %', current_crit_level, p_crit_level_delta;
        END IF;
        IF next_crit_level > 4294967295 THEN
            RAISE EXCEPTION 'crit_level overflow: %', next_crit_level;
        END IF;

        next_armor_level := current_armor_level + p_armor_level_delta;
        IF next_armor_level < 0 THEN
            RAISE EXCEPTION 'armor_level cannot go negative: % + %', current_armor_level, p_armor_level_delta;
        END IF;
        IF next_armor_level > 4294967295 THEN
            RAISE EXCEPTION 'armor_level overflow: %', next_armor_level;
        END IF;

        next_max_health_level := current_max_health_level + p_max_health_level_delta;
        IF next_max_health_level < 0 THEN
            RAISE EXCEPTION 'max_health_level cannot go negative: % + %', current_max_health_level, p_max_health_level_delta;
        END IF;
        IF next_max_health_level > 4294967295 THEN
            RAISE EXCEPTION 'max_health_level overflow: %', next_max_health_level;
        END IF;

        weapon_state := jsonb_build_object(
            'fragments', next_fragments,
            'damage_level', next_damage_level,
            'attack_speed_level', next_attack_speed_level,
            'pierce_level', next_pierce_level,
            'crit_level', next_crit_level,
            'armor_level', next_armor_level,
            'max_health_level', next_max_health_level
        );

        current_state := jsonb_set(
            COALESCE(current_state, '{"version":6,"equipped_weapon_id":0,"weapons":{}}'::JSONB),
            ARRAY['weapons', weapon_key],
            weapon_state,
            true
        );
    END IF;

    IF p_set_equipped THEN
        current_state := jsonb_set(
            COALESCE(current_state, '{"version":6,"equipped_weapon_id":0,"weapons":{}}'::JSONB),
            '{equipped_weapon_id}',
            to_jsonb(p_weapon_id),
            true
        );
    END IF;

    current_state := jsonb_set(current_state, '{version}', '6'::JSONB, true);

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
            COALESCE((change->>'fragment_delta')::BIGINT, 0),
            COALESCE(
                (change->>'damage_level_delta')::BIGINT,
                (change->>'level_delta')::BIGINT,
                0
            ),
            COALESCE((change->>'attack_speed_level_delta')::BIGINT, 0),
            COALESCE((change->>'pierce_level_delta')::BIGINT, 0),
            COALESCE((change->>'crit_level_delta')::BIGINT, 0),
            COALESCE((change->>'armor_level_delta')::BIGINT, 0),
            COALESCE((change->>'max_health_level_delta')::BIGINT, 0),
            COALESCE((change->>'set_equipped')::BOOLEAN, false)
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
REVOKE ALL ON FUNCTION apply_player_change(TEXT, INT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BOOLEAN) FROM PUBLIC;
REVOKE ALL ON FUNCTION apply_persistence_transaction(UUID, JSONB) FROM PUBLIC;
REVOKE ALL ON FUNCTION get_player_states(TEXT[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION flush_player_transactions(JSONB, TEXT[]) FROM PUBLIC;

REVOKE ALL ON FUNCTION player_state_hash(TEXT, JSONB) FROM inventory_jam_server;
REVOKE ALL ON FUNCTION ensure_player(TEXT) FROM inventory_jam_server;
REVOKE ALL ON FUNCTION apply_player_change(TEXT, INT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BIGINT, BOOLEAN) FROM inventory_jam_server;
REVOKE ALL ON FUNCTION apply_persistence_transaction(UUID, JSONB) FROM inventory_jam_server;

-- Server may only call the two batch API functions.
GRANT EXECUTE ON FUNCTION get_player_states(TEXT[]) TO inventory_jam_server;
GRANT EXECUTE ON FUNCTION flush_player_transactions(JSONB, TEXT[]) TO inventory_jam_server;

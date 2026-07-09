-- Server discovery for game jam clients on DigitalOcean Postgres (database: inventory_jam).
-- Users inventory_jam_client and inventory_jam_server are created in the DO control panel.
-- Clients call get_latest_server() only. Deploy scripts call set_latest_server().

CREATE TABLE IF NOT EXISTS server_registry (
    id int PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    host text NOT NULL,
    port int NOT NULL CHECK (port > 0 AND port <= 65535),
    updated_at timestamptz NOT NULL DEFAULT now()
);

REVOKE ALL ON server_registry FROM PUBLIC;

CREATE OR REPLACE FUNCTION set_latest_server(p_host text, p_port int)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    INSERT INTO server_registry (id, host, port, updated_at)
    VALUES (1, p_host, p_port, now())
    ON CONFLICT (id) DO UPDATE SET
        host = EXCLUDED.host,
        port = EXCLUDED.port,
        updated_at = EXCLUDED.updated_at;
END;
$$;

CREATE OR REPLACE FUNCTION get_latest_server()
RETURNS TABLE(host text, port int)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
    SELECT server_registry.host, server_registry.port
    FROM server_registry
    WHERE server_registry.id = 1;
$$;

REVOKE ALL ON FUNCTION set_latest_server(text, int) FROM PUBLIC;
REVOKE ALL ON FUNCTION get_latest_server() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION get_latest_server() TO inventory_jam_client;
GRANT EXECUTE ON FUNCTION set_latest_server(text, int) TO inventory_jam_server;

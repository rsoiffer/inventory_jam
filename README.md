# Simple box

A simple example that shows how to use Lightyear to create a server-authoritative multiplayer game.

It also showcases how to enable client-side prediction and snapshot interpolation:

- For the client sending inputs: the pink cube is client-predicted (so inputs are used with no delay, and there is a
  rollback in case of mismatch with the server) and the red cube shows the received server state. (the server state
  arrives with some delay, and is a bit choppy since the replication rate is only 10Hz).
- For the other clients: the red cube still shows the server states arriving at 10Hz, and the pink cube is a smooth
  interpolation between those states (there is a slight delay because we can only interpolate between 2 received server
  states).

https://github.com/cBournhonesque/lightyear/assets/8112632/7b57d48a-d8b0-4cdd-a16f-f991a394c852

## Running locally

- Run the server with a gui: `cargo run -- server`
- Run client with id 1: `cargo run -- client -c 1`
- Run the server without a gui: `cargo run --no-default-features --features=server -- server`
- Run a headless client without a gui:
  `cargo run --no-default-features --features=client,netcode,webtransport -- client -c 1`
- Run the client and server in "HostClient" mode: `cargo run -- host-client -c 0`

You can control the behaviour of the example by changing the list of features. By default, all features are enabled (
client, server, gui, server-discovery).
For example you can run the server in headless mode (without gui) by running
`cargo run --no-default-features --features=server,webtransport,netcode`.

### Client configuration

Edit hardcoded client settings in [`src/networking.rs`](src/networking.rs) and [
`src/server_discovery.rs`](src/server_discovery.rs):

- `CERT_DIGEST` in `networking.rs` — WebTransport certificate fingerprint from server deployment logs
- `USE_SERVER_DISCOVERY` in `server_discovery.rs` — set `false` for local UDP against `127.0.0.1:5888`
- `DATABASE_URL` in `server_discovery.rs` — read-only Postgres user that can call `get_latest_server()`

Then run the client normally:

```powershell
cargo run -- client -c 1
```

## Edgegap dedicated servers

The dedicated server image runs headless with WebTransport and reads Edgegap injected variables at runtime:

- `ARBITRIUM_PORT_GAMEPORT_INTERNAL` — bind port inside the container
- `ARBITRIUM_PUBLIC_IP` / `ARBITRIUM_PORT_GAMEPORT_EXTERNAL` — public connection details
- `ARBITRIUM_REQUEST_ID` — deployment id (`{request_id}.pr.edgegap.net`)

Configure your Edgegap app version with a `gameport` mapped to the internal listen port (default `5888`) and protocol
`UDP` (WebTransport/QUIC).

### Database server discovery

Clients call a single Postgres function on DigitalOcean Postgres (`inventory_jam` database) to find the latest Edgegap
server:

```sql
SELECT host, port FROM get_latest_server();
```

Setup:

```powershell
Copy-Item secrets/database-admin-url.txt.example secrets/database-admin-url.txt
Copy-Item secrets/database-client-url.txt.example secrets/database-client-url.txt
Copy-Item secrets/database-server-url.txt.example secrets/database-server-url.txt
# replace PASSWORD in each file

# One-time schema setup (doadmin only):
.\scripts\deploy-sql.ps1
```

Secret files:

- `database-admin-url.txt` — `doadmin`, for one-time SQL setup only
- `database-server-url.txt` — `inventory_jam_server`, used by the deploy script
- `database-client-url.txt` — `inventory_jam_client`, share with other devs for the game client

- `inventory_jam_client` — can execute `get_latest_server()` only
- `inventory_jam_server` — can execute `set_latest_server(host, port)` only

Paste the client URL into `DATABASE_URL` in `server_discovery.rs` (or set the `DATABASE_URL` env var).

### Deploy script

Edit the hardcoded values at the top of [`scripts/deploy-edgegap.ps1`](scripts/deploy-edgegap.ps1) (`$AppName`,
`$AppVersion`, `$Registry`, etc.), then run:

```powershell
Copy-Item secrets/edgegap-token.txt.example secrets/edgegap-token.txt
Copy-Item secrets/registry-credentials.txt.example secrets/registry-credentials.txt
Copy-Item secrets/database-server-url.txt.example secrets/database-server-url.txt
# paste your tokens/credentials on the first line(s) of each file
.\scripts\deploy-edgegap.ps1
```

The API token is read from `secrets/edgegap-token.txt` (token only, one line), or from the `EDGEGAP_API_TOKEN`
environment variable in CI.
Registry credentials for push and image verification are read from `secrets/registry-credentials.txt` (username on
line 1, token on line 2), or from `EDGEGAP_REGISTRY_USERNAME` and `EDGEGAP_REGISTRY_TOKEN` in CI. Get these from the
Edgegap dashboard under **Container Registry**.
The deploy database URL is read from `secrets/database-server-url.txt`, or from `DATABASE_SERVER_URL` in CI.

Flow:

1. Build and push the Docker image
2. Create the app version if missing, otherwise `PATCH` the existing version's image reference
3. Stop any active deployments, then `POST /v2/deployments` near Portland, OR
4. Poll until the deployment is ready, then call `set_latest_server(public_ip, external_port)` via `psql`

Flags: `-SkipBuild`, `-SkipPush`, `-SkipVersionUpdate`, `-SkipDeploy`, `-SkipDatabaseUpdate`

### Local container smoke test

```powershell
docker build -t inventory-jam-server .
docker run --rm -p 5888:5888/udp `
  -e ARBITRIUM_REQUEST_ID=localtest `
  -e ARBITRIUM_PUBLIC_IP=127.0.0.1 `
  -e ARBITRIUM_PORT_GAMEPORT_INTERNAL=5888 `
  -e ARBITRIUM_PORT_GAMEPORT_EXTERNAL=5888 `
  inventory-jam-server
```

Connect a local client with WebTransport (set `USE_SERVER_DISCOVERY = false` in `server_discovery.rs`, update
`CERT_DIGEST` in `networking.rs`, and insert the local address with `SELECT set_latest_server('127.0.0.1', 5888);`):

```powershell
cargo run --no-default-features --features=client,netcode,webtransport,server-discovery -- client -c 1
```

### Testing in wasm with webtransport

NOTE: I am using the [bevy cli](https://github.com/TheBevyFlock/bevy_cli) to build and serve the wasm example.

To test the example in wasm, you can run the following commands: `bevy run web`

Pass the certificate digest via URL hash (`#<digest>`) or `window.CERT_DIGEST`. Regenerate local certificates when
needed and update `certificates/digest.txt` before rebuilding wasm clients.

Server discovery via Postgres is native-only (`server-discovery` feature).

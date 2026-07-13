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

GUI builds open a **main menu** by default. Enter a username, then:

- **Join** — discover/connect via Edgegap
- **Host Local Server** / **Join Local Server** — available in the default development build

Skip the menu with CLI flags:

- Dedicated server: `cargo run -- --mode server`
- Join Edgegap: `cargo run -- --mode edgegap --user Ada`
- Host local: `cargo run -- --mode host`
- Join local: `cargo run -- --mode local --user Ada`

Netcode client ids are derived from `--user` / the menu username, so each player needs a distinct name.

Headless:

- Server: `cargo run --no-default-features --features=server,netcode,webtransport -- --mode server`
- Client: `cargo run --no-default-features --features=client,netcode,webtransport -- --mode edgegap --user Ada`

Local multi-client smoke test: `just dev` / `just dev 3` (starts a local host plus N clients).
Local-join usernames are automatically prefixed with a per-machine id in Rust so they
do not collide with other developers sharing the same persistence database.

The default features enable client, server, GUI, development tools, UDP, netcode, and WebTransport. Production and
headless builds should use `--no-default-features` with only the required features.

### Client configuration

Hardcoded client settings live in [`src/networking/`](src/networking/):

- `CERT_DIGEST` in `setup.rs` — WebTransport certificate fingerprint
- `--mode local` — connect with local UDP to `127.0.0.1:5888` instead of Edgegap
- Edgegap API token and app name/version in `edgegap.rs`

Menu Join (no flags):

```powershell
cargo run
```

## Edgegap dedicated servers

The dedicated server image runs headless with WebTransport and reads Edgegap injected variables at runtime:

- `ARBITRIUM_PORT_GAMEPORT_INTERNAL` — bind port inside the container
- `ARBITRIUM_PUBLIC_IP` / `ARBITRIUM_PORT_GAMEPORT_EXTERNAL` — public connection details
- `ARBITRIUM_REQUEST_ID` — deployment id (`{request_id}.pr.edgegap.net`)

Configure your Edgegap app version with a `gameport` mapped to the internal listen port (default `5888`) and protocol
`UDP` (WebTransport/QUIC).

### Direct Edgegap discovery

When joining via the menu **Join** button or `--mode edgegap`, clients call the Edgegap API:

1. `GET /v1/deployments` — find a running `inventory-jam@dev` deployment
2. If none is available, `POST /v2/deployments` — start a new deployment near Portland, OR
3. Poll `GET /v1/status/{request_id}` until `running == true`
4. Connect with WebTransport to the returned `gameport` endpoint

### Deploy script

Edit the hardcoded values at the top of [`scripts/deploy-edgegap.ps1`](scripts/deploy-edgegap.ps1) (`$AppName`,
`$AppVersion`, `$Registry`, `$ApiToken`, registry credentials, etc.), then run:

```powershell
.\scripts\deploy-edgegap.ps1
```

Flow:

1. Build and push the Docker image with a unique `dev-<timestamp>` tag
2. Create the app version if missing, otherwise `PATCH` the existing version's image reference
3. Stop any active deployments, then `POST /v2/deployments` near Portland, OR
4. Poll until the deployment is ready

Flags: `-SkipBuild`, `-SkipPush`, `-SkipVersionUpdate`, `-SkipDeploy`, `-ImageTag <tag>`. If you skip build or push, pass
`-ImageTag` explicitly so the script does not point Edgegap at an unbuilt tag.

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

### Testing in wasm with webtransport

```powershell
trunk serve
```

Open http://127.0.0.1:8080/. Trunk builds with `client,gui,netcode,webtransport` (no server).

If `trunk` fails with an invalid `--no-color` value, clear `NO_COLOR` first:

```powershell
$env:NO_COLOR = $null
trunk serve
```

Firefox currently needs a local `xwt-web` patch that avoids BYOB readers for WebTransport datagrams. Background:
[Mozilla bug 2007755](https://bugzilla.mozilla.org/show_bug.cgi?id=2007755) and
[MOZGIII/xwt issue 156](https://github.com/MOZGIII/xwt/issues/156).

Native and wasm clients use the same async Edgegap discovery path before connecting.

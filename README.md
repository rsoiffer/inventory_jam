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
client, server, gui).
For example you can run the server in headless mode (without gui) by running
`cargo run --no-default-features --features=server,webtransport,netcode`.

### Client configuration

Edit hardcoded client settings in [`src/networking.rs`](src/networking.rs) and [
`src/server_discovery.rs`](src/server_discovery.rs):

- `CERT_DIGEST` in `networking.rs` — WebTransport certificate fingerprint from server deployment logs
- `USE_LOCAL_SERVER` in `server_discovery.rs` — set `true` for local UDP against `127.0.0.1:5888`

Remote clients discover or start an Edgegap deployment directly through the Edgegap API using the same code path on
native and wasm builds. The API token is hardcoded in `server_discovery.rs` for this game jam build.

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

### Direct Edgegap discovery

On startup, clients call the Edgegap API directly:

1. `GET /v1/deployments` — find a running `inventory-jam@dev` deployment
2. If none is available, `POST /v2/deployments` — start a new deployment near Portland, OR
3. Poll `GET /v1/status/{request_id}` until `running == true`
4. Connect with WebTransport to the returned `gameport` endpoint

### Deploy script

Edit the hardcoded values at the top of [`scripts/deploy-edgegap.ps1`](scripts/deploy-edgegap.ps1) (`$AppName`,
`$AppVersion`, `$Registry`, etc.), then run:

```powershell
Copy-Item secrets/edgegap-token.txt.example secrets/edgegap-token.txt
Copy-Item secrets/registry-credentials.txt.example secrets/registry-credentials.txt
# paste your tokens/credentials on the first line(s) of each file
.\scripts\deploy-edgegap.ps1
```

The API token is read from `secrets/edgegap-token.txt` (token only, one line), or from the `EDGEGAP_API_TOKEN`
environment variable in CI.
Registry credentials for push and image verification are read from `secrets/registry-credentials.txt` (username on
line 1, token on line 2), or from `EDGEGAP_REGISTRY_USERNAME` and `EDGEGAP_REGISTRY_TOKEN` in CI. Get these from the
Edgegap dashboard under **Container Registry**.

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

Connect a local client with WebTransport (set `USE_LOCAL_SERVER = true` in `server_discovery.rs` and update
`CERT_DIGEST` in `networking.rs`):

```powershell
cargo run --no-default-features --features=client,netcode,webtransport -- client -c 1
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

The wasm client reads the WebTransport certificate digest from `certificates/digest.txt`.

Firefox currently needs a local `xwt-web` patch that avoids BYOB readers for WebTransport datagrams. Background:
[Mozilla bug 2007755](https://bugzilla.mozilla.org/show_bug.cgi?id=2007755) and
[MOZGIII/xwt issue 156](https://github.com/MOZGIII/xwt/issues/156).

Native and wasm clients use the same async Edgegap discovery path before connecting.

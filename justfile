# Install with: cargo install just

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

[unix]
dev n="2":
    cargo build
    cargo run -- --mode host &
    sleep 1
    for i in $(seq 1 {{n}}); do \
        cargo run -- --mode local --user "Player$i" & \
    done

[windows]
dev n="2":
    cargo build
    Start-Process -NoNewWindow cargo -ArgumentList @('run','--','--mode','host')
    Start-Sleep -Seconds 1
    1..{{n}} | ForEach-Object { Start-Process -NoNewWindow cargo -ArgumentList @('run','--','--mode','local','--user',"Player$_") }

# Check the project builds cleanly under every client/server/gui combo
[unix]
check-features:
    cargo clippy --no-default-features --all-targets --features "netcode,udp,webtransport,client,gui"
    cargo clippy --no-default-features --all-targets --features "netcode,udp,webtransport,server"
    cargo clippy --no-default-features --all-targets --features "netcode,udp,webtransport,client,server,gui"

# Check the project builds cleanly under every client/server/gui combo
[windows]
check-features:
    cargo clippy --no-default-features --all-targets --features "netcode,udp,webtransport,client,gui"
    cargo clippy --no-default-features --all-targets --features "netcode,udp,webtransport,server"
    cargo clippy --no-default-features --all-targets --features "netcode,udp,webtransport,client,server,gui"

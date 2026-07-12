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

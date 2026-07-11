# The 'Just' utility lets you run common project actions.
# Install with: cargo install just
# Usage: just dev
#        just dev 3

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

[unix]
dev n="2":
    cargo run -- server &
    for i in $(seq 1 {{n}}); do \
        cargo run -- client -c $i --dev & \
    done

[windows]
dev n="2":
    Start-Process -NoNewWindow cargo -ArgumentList @('run','--','server')
    1..{{n}} | ForEach-Object { Start-Process -NoNewWindow cargo -ArgumentList @('run','--','client','-c',$_,'--dev') }

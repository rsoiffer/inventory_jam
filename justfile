# The 'Just' utility let's you run common project actions
# Install Just with `$ cargo install just`
# Usage: `$ just dev`

dev n="2":
    cargo run -- server &
    for i in $(seq 1 {{n}}); do \
        cargo run -- client -c $i & \
    done;

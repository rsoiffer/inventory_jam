# The 'Just' utility let's you run common project actions
# Install Just with `$ cargo install just`
# Usage: `$ just dev`

dev:
    cargo build
    cargo run -- server &
    cargo run -- client -c 1 &
    cargo run -- client -c 2

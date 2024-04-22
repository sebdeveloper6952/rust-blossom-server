dev:
    cargo watch -x check -x run

check:
    cargo watch -x check

lint:
    cargo clippy -- -D warnings

fmt:
    cargo fmt -- --check

test:
    cargo test

ma NAME:
    sqlx migrate add --source db/migrations -s {{NAME}}

mr:
    sqlx migrate run --source db/migrations

run:
    RUST_LOG=rust_blossom_server=TRACE cargo run

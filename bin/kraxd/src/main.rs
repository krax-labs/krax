//! Krax sequencer node binary (`kraxd`).

// Per AGENTS.md Rule 4, runtime output goes through `tracing`. The version
// banner is a startup UX contract (Phase 0 Gate: `make run` prints version),
// not a log event — so `println!` is intentional here. tracing-subscriber
// initialization arrives in a later step alongside krax-config.
fn main() {
    println!("krax v{}", env!("CARGO_PKG_VERSION"));
}

// Per Context7 (clap 4.x, docs.rs/clap/latest, May 2026):
// `CommandFactory::command()` accesses the underlying `Command` from a
// `Parser` derive — source: docs.rs/clap/latest/clap/builder/struct.Command.html
// ("When deriving a Parser, you can use CommandFactory::command to access the
// Command"). `Command::print_help()` prints short help to stdout (same source).
// `#[derive(Subcommand)]` on a zero-variant enum compiles correctly;
// `Option<Commands>` is always `None` until variants are added.
use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kraxctl", about = "Krax operator CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

// No variants yet. Subcommand variants land in the step that introduces each
// operator command (init, status, etc.). The empty enum establishes the
// structural skeleton now so main.rs does not need restructuring later.
#[derive(Subcommand)]
enum Commands {}

fn main() {
    let cli = Cli::parse();
    if cli.command.is_none() {
        <Cli as CommandFactory>::command()
            .print_help()
            .expect("failed to write help to stdout");
    }
}

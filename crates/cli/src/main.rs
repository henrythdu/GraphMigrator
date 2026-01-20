use clap::Parser;

/// GraphMigrator - Visual task-tracking system for code migration
#[derive(Parser)]
#[command(name = "migrator")]
#[command(author = "Henry Du")]
#[command(version)] // Auto-pull version from Cargo.toml
#[command(about = "Transform codebases into queryable dependency graphs", long_about = None)]
struct Cli;

fn main() {
    let _cli = Cli::parse();
    // Clap handles --version and --help automatically
    // For now, just print a message to verify the CLI works
    println!(
        "GraphMigrator CLI v{} - Workspace initialized!",
        env!("CARGO_PKG_VERSION")
    );
}

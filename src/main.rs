use anyhow::Context;
use purge_deps::{cli, run, Config};

/// Binary entry: parse argv, then run the library.
///
/// Example: `purge-deps -p ./apps -e dist` parses flags, builds config, and purges.
fn main() -> anyhow::Result<()> {
    let config = Config::from(cli::parse());
    run(config).context("failed to purge dependencies")?;
    Ok(())
}

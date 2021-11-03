#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]

mod cli;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args = cli::args().with_context(|| "parsing CLI args")?;

    let filesystem = mmlspool::run(&args.filesystem)?;

    let pool = filesystem
        .pools()
        .iter()
        .find(|pool| pool.name() == args.pool)
        .with_context(|| format!("pool {} not found", args.pool))?;

    let data_pool_size = pool
        .data()
        .with_context(|| format!("pool {} is not object data", args.pool))?;

    println!("{}", data_pool_size.used_percent());

    Ok(())
}

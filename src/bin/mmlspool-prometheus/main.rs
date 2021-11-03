#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]

mod cli;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args = cli::args().with_context(|| "parsing CLI args")?;

    let filesystems = mmlspool::run_all(&args.fs_names)?;

    let prom = mmlspool::to_prom(&filesystems)
        .with_context(|| "converting internal data to prometheus")?;

    print!("{}", prom);

    Ok(())
}

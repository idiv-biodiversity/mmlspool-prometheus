#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]

mod cli;

use std::borrow::Cow;
use std::io::prelude::*;

use anyhow::{Context, Result};

use cli::Arguments;

fn main() -> Result<()> {
    let args = cli::args().with_context(|| "parsing CLI args")?;

    let fs_names = fs_names_args_or_stdin(&args)?;

    if !fs_names.is_empty() {
        let filesystems = mmlspool::run_all(&fs_names)?;

        let prom = mmlspool::to_prom(&filesystems)
            .with_context(|| "converting internal data to prometheus")?;

        print!("{}", prom);
    }

    Ok(())
}

/// Returns file system names from given arguments, if any were provided,
/// otherwise reads file system names from **STDIN** and returns these.
///
/// As file system names should not be empty, these are automatically filtered.
fn fs_names_args_or_stdin(args: &Arguments) -> Result<Vec<Cow<str>>> {
    let mut fs_names: Vec<Cow<str>> = vec![];

    if let Some(names) = &args.fs_names {
        fs_names.extend(names.iter().map(Cow::from));
    } else {
        let stdin = std::io::stdin();

        for line in stdin.lock().lines() {
            let line =
                line.with_context(|| "error reading line from STDIN")?;

            fs_names.push(Cow::from(line));
        }
    };

    let fs_names = fs_names
        .into_iter()
        .filter(|name| !name.is_empty())
        .collect();

    Ok(fs_names)
}

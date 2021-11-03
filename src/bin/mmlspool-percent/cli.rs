use anyhow::{Context, Result};
use clap::crate_version;
use clap::{App, Arg, ArgMatches};

#[derive(Debug)]
pub struct Arguments {
    pub filesystem: String,
    pub pool: String,
}

impl TryFrom<ArgMatches> for Arguments {
    type Error = anyhow::Error;

    fn try_from(args: ArgMatches) -> Result<Self, Self::Error> {
        let filesystem = args
            .value_of("filesystem")
            .with_context(|| "no filesystem argument")?
            .into();

        let pool = args
            .value_of("pool")
            .with_context(|| "no pool argument")?
            .into();

        Ok(Self { filesystem, pool })
    }
}

pub fn args() -> Result<Arguments> {
    let arguments = build().get_matches();
    let arguments = Arguments::try_from(arguments)?;
    Ok(arguments)
}

pub fn build() -> App<'static> {
    let fs = Arg::new("filesystem")
        .takes_value(true)
        .required(true)
        .about("file system");

    let pool = Arg::new("pool")
        .takes_value(true)
        .required(true)
        .about("pool name");

    App::new("mmlspool-percent")
        .about("show pool used in percent")
        .version(crate_version!())
        .arg(fs)
        .arg(pool)
        .mut_arg("help", |a| {
            a.short('?').about("print help").long_about("Print help.")
        })
        .mut_arg("version", |a| {
            a.hidden_short_help(true).long_about("Print version.")
        })
}

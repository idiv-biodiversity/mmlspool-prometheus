use anyhow::Result;
use clap::crate_version;
use clap::{App, Arg, ArgMatches};

#[derive(Debug)]
pub struct Arguments {
    pub fs_names: Option<Vec<String>>,
}

impl TryFrom<ArgMatches> for Arguments {
    type Error = anyhow::Error;

    fn try_from(args: ArgMatches) -> Result<Self, Self::Error> {
        let fs = args
            .values_of("filesystems")
            .map(|v| v.map(Into::into).collect::<Vec<String>>());

        Ok(Self { fs_names: fs })
    }
}

pub fn args() -> Result<Arguments> {
    let arguments = build().get_matches();
    let arguments = Arguments::try_from(arguments)?;
    Ok(arguments)
}

pub fn build() -> App<'static> {
    let fs = Arg::new("filesystems")
        .takes_value(true)
        .multiple_values(true)
        .about("file systems")
        .long_about("File systems. If not specified, reads from STDIN.");

    App::new("mmlspool-prometheus")
        .about("mmlspool to prometheus")
        .version(crate_version!())
        .arg(fs)
        .mut_arg("help", |a| {
            a.short('?').about("print help").long_about("Print help.")
        })
        .mut_arg("version", |a| {
            a.hidden_short_help(true).long_about("Print version.")
        })
}

//! CLI argument definitions and localized parsing helpers.

mod args;
mod i18n;

use std::ffi::OsString;

use clap::{CommandFactory, FromArgMatches};

use crate::i18n::Language;

pub use args::*;

#[derive(Debug)]
pub struct ParsedCli {
    pub cli: Cli,
    pub lang: Language,
}

#[derive(Debug)]
pub struct CliParseError {
    error: clap::Error,
    lang: Language,
}

impl CliParseError {
    pub fn kind(&self) -> clap::error::ErrorKind {
        self.error.kind()
    }

    pub fn use_stderr(&self) -> bool {
        self.error.use_stderr()
    }

    pub fn exit_code(&self) -> i32 {
        self.error.exit_code()
    }

    pub fn rendered(&self) -> String {
        i18n::render_clap_error(&self.error, self.lang)
    }

    pub fn message(&self) -> String {
        self.error.to_string()
    }
}

pub fn parse_env() -> ParsedCli {
    parse_from(std::env::args_os()).unwrap_or_else(|error| {
        let rendered = error.rendered();
        if error.use_stderr() {
            eprint!("{rendered}");
        } else {
            print!("{rendered}");
        }
        std::process::exit(error.exit_code());
    })
}

pub fn parse_from<I, T>(args: I) -> Result<ParsedCli, CliParseError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<OsString>>();
    let lang = i18n::resolve_cli_language(&args);
    let matches = command_for_sources(&args)
        .try_get_matches_from(args)
        .map_err(|error| CliParseError { error, lang })?;
    let cli = Cli::from_arg_matches(&matches)
        .expect("clap matches should deserialize into the generated CLI type");

    Ok(ParsedCli { cli, lang })
}

pub(crate) fn command_for_sources(args: &[OsString]) -> clap::Command {
    let lang = i18n::resolve_cli_language(args);
    i18n::localize_command(Cli::command(), lang).color(i18n::default_cli_color())
}

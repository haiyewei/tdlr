use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    io::IsTerminal,
    sync::OnceLock,
};

use clap::{Arg, ArgAction, ColorChoice, Command};
use serde::Deserialize;

use crate::i18n::{normalize_language_tag, Language};

const DEFAULT_LOCALE: Language = Language::EnUs;
const DEFAULT_CLI_COLOR: ColorChoice = ColorChoice::Auto;
const EMBEDDED_CATALOGS: &[&str] = &[
    include_str!("../../locales/cli/zh-CN.json"),
    include_str!("../../locales/cli/en-US.json"),
];

pub(crate) fn resolve_cli_language(args: &[OsString]) -> Language {
    detect_arg_language(args)
        .or_else(|| {
            std::env::var("TDLR_LANG")
                .ok()
                .as_deref()
                .and_then(Language::parse_raw)
        })
        .or_else(detect_locale_language)
        .or_else(detect_windows_ui_language)
        .unwrap_or(DEFAULT_LOCALE)
}

pub(crate) fn localize_command(command: Command, lang: Language) -> Command {
    localize_root(command, catalog_for(lang))
}

pub(crate) fn render_clap_error(error: &clap::Error, lang: Language) -> String {
    let rendered = if should_render_ansi(error.use_stderr()) {
        error.render().ansi().to_string()
    } else {
        error.to_string()
    };
    localize_clap_error_message(&rendered, lang)
}

pub(crate) fn default_cli_color() -> ColorChoice {
    DEFAULT_CLI_COLOR
}

fn localize_root(command: Command, catalog: &'static LocaleCatalog) -> Command {
    let command = command
        .mut_subcommand("auth", |sub| localize_auth(sub, catalog))
        .mut_subcommand("upload", |sub| {
            standard_command(sub, catalog, "upload", catalog.headings.options.clone())
        })
        .mut_subcommand("download", |sub| {
            standard_command(sub, catalog, "download", catalog.headings.options.clone())
        })
        .mut_subcommand("forward", |sub| {
            standard_command(sub, catalog, "forward", catalog.headings.options.clone())
        })
        .mut_subcommand("service", |sub| {
            standard_command(sub, catalog, "service", catalog.headings.options.clone())
        });

    standard_command(
        command,
        catalog,
        "root",
        catalog.headings.global_options.clone(),
    )
    .version(leak_str(&version_output(catalog)))
    .long_version(leak_str(&version_output(catalog)))
    .after_help(catalog.help.root_after_help.clone())
}

fn localize_auth(command: Command, catalog: &'static LocaleCatalog) -> Command {
    let command = command
        .mut_subcommand("login", |sub| localize_auth_login(sub, catalog))
        .mut_subcommand("logout", |sub| {
            standard_command(
                sub,
                catalog,
                "auth.logout",
                catalog.headings.options.clone(),
            )
        })
        .mut_subcommand("status", |sub| {
            standard_command(
                sub,
                catalog,
                "auth.status",
                catalog.headings.options.clone(),
            )
        });

    standard_command(command, catalog, "auth", catalog.headings.options.clone())
}

fn localize_auth_login(command: Command, catalog: &'static LocaleCatalog) -> Command {
    let command = command
        .mut_subcommand("add", |sub| {
            standard_command(
                sub,
                catalog,
                "auth.login.add",
                catalog.headings.options.clone(),
            )
        })
        .mut_subcommand("list", |sub| {
            standard_command(
                sub,
                catalog,
                "auth.login.list",
                catalog.headings.options.clone(),
            )
        })
        .mut_subcommand("remove", |sub| {
            standard_command(
                sub,
                catalog,
                "auth.login.remove",
                catalog.headings.options.clone(),
            )
        })
        .mut_subcommand("use", |sub| {
            standard_command(
                sub,
                catalog,
                "auth.login.use",
                catalog.headings.options.clone(),
            )
        });

    standard_command(
        command,
        catalog,
        "auth.login",
        catalog.headings.options.clone(),
    )
}

fn standard_command(
    command: Command,
    catalog: &'static LocaleCatalog,
    key: &str,
    options_heading_text: String,
) -> Command {
    let mut command = command
        .about(catalog.command_about(key).to_owned())
        .long_about(None)
        .help_template(help_template(catalog))
        .subcommand_help_heading(leak_str(&catalog.headings.commands))
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .arg(custom_help_arg(catalog, &options_heading_text));

    if key == "root" {
        command = command
            .disable_version_flag(true)
            .arg(custom_version_arg(catalog, &options_heading_text));
    }

    localize_known_args(command, catalog, key, options_heading_text)
}

fn localize_known_args(
    mut command: Command,
    catalog: &'static LocaleCatalog,
    command_key: &str,
    options_heading_text: String,
) -> Command {
    for id in catalog.arg_ids() {
        let Some(is_positional) = arg_is_positional(&command, id) else {
            continue;
        };

        let heading = if is_positional {
            catalog.headings.arguments.clone()
        } else {
            options_heading_text.clone()
        };
        let help = catalog.arg_help(command_key, id).to_owned();
        let hide_possible_values = catalog.has_command_arg_override(command_key, id)
            && arg_has_possible_values(&command, id);

        command = command.mut_arg(id, move |arg| {
            let arg = arg
                .help(help.clone())
                .help_heading(Some(leak_str(&heading)));
            let arg = if arg.get_env().is_some() {
                arg.hide_env(true)
            } else {
                arg
            };

            if hide_possible_values {
                arg.hide_possible_values(true)
            } else {
                arg
            }
        });
    }

    command
}

fn custom_help_arg(catalog: &LocaleCatalog, heading: &str) -> Arg {
    Arg::new("help")
        .short('h')
        .long("help")
        .action(ArgAction::Help)
        .help(leak_str(catalog.arg_help("root", "help")))
        .help_heading(Some(leak_str(heading)))
}

fn custom_version_arg(catalog: &LocaleCatalog, heading: &str) -> Arg {
    Arg::new("version")
        .short('V')
        .long("version")
        .action(ArgAction::Version)
        .help(leak_str(catalog.arg_help("root", "version")))
        .help_heading(Some(leak_str(heading)))
}

fn help_template(catalog: &LocaleCatalog) -> String {
    format!(
        "{{before-help}}{{name}} {}\n{{about-with-newline}}{}: {{usage}}\n\n{{all-args}}{{after-help}}",
        env!("TDLR_VERSION"),
        catalog.headings.usage,
    )
}

fn version_output(catalog: &LocaleCatalog) -> String {
    let labels = version_labels(catalog);

    format!(
        "{}\n{}: {}\n{}: {}\n{}: {}/{}",
        env!("TDLR_VERSION"),
        labels.version,
        env!("TDLR_VERSION"),
        labels.rustc,
        env!("RUSTC_VERSION"),
        labels.target,
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

fn version_labels(catalog: &LocaleCatalog) -> VersionLabels {
    if catalog.meta.code == "zh-CN" {
        VersionLabels {
            version: "版本",
            rustc: "Rustc",
            target: "目标平台",
        }
    } else {
        VersionLabels {
            version: "Version",
            rustc: "Rustc",
            target: "Target",
        }
    }
}

fn arg_is_positional(command: &Command, id: &str) -> Option<bool> {
    command
        .get_arguments()
        .find(|arg| arg.get_id().as_str() == id)
        .map(|arg| arg.get_index().is_some())
}

fn arg_has_possible_values(command: &Command, id: &str) -> bool {
    command
        .get_arguments()
        .find(|arg| arg.get_id().as_str() == id)
        .map(|arg| !arg.get_possible_values().is_empty())
        .unwrap_or(false)
}

fn should_render_ansi(use_stderr: bool) -> bool {
    match DEFAULT_CLI_COLOR {
        ColorChoice::Always => true,
        ColorChoice::Auto => {
            if use_stderr {
                std::io::stderr().is_terminal()
            } else {
                std::io::stdout().is_terminal()
            }
        }
        ColorChoice::Never => false,
    }
}

fn detect_arg_language(args: &[OsString]) -> Option<Language> {
    let mut index = 1usize;
    while index < args.len() {
        let arg = args[index].to_string_lossy();

        if arg == "--lang" {
            return args
                .get(index + 1)
                .and_then(|value| Language::parse_raw(&value.to_string_lossy()));
        }

        if let Some(value) = arg.strip_prefix("--lang=") {
            return Language::parse_raw(value);
        }

        index += 1;
    }

    None
}

fn detect_locale_language() -> Option<Language> {
    ["LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok())
        .as_deref()
        .and_then(Language::parse_raw)
}

#[cfg(windows)]
fn detect_windows_ui_language() -> Option<Language> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }

    let lang_id = unsafe { GetUserDefaultUILanguage() };
    let primary = lang_id & 0x03ff;

    match primary {
        0x0004 => Some(Language::ZhCn),
        0x0009 => Some(Language::EnUs),
        _ => None,
    }
}

#[cfg(not(windows))]
fn detect_windows_ui_language() -> Option<Language> {
    None
}

pub(crate) fn localize_clap_error_message(message: &str, lang: Language) -> String {
    match lang {
        Language::ZhCn => {
            let mut localized = message.to_owned();
            for (from, to) in [
                (
                    "For more information, try '--help'.",
                    "如需更多信息，请使用 '--help'。",
                ),
                (
                    "the following required arguments were not provided:",
                    "缺少以下必需参数：",
                ),
                ("a value is required for", "以下参数需要一个值："),
                ("but none was supplied", "但未提供值"),
                ("unexpected argument '", "发现意外参数 '"),
                ("' found", "'"),
                (
                    "' requires a subcommand but one was not provided",
                    "' 需要一个子命令，但未提供",
                ),
                ("unrecognized subcommand", "无法识别的子命令"),
                ("unexpected argument", "发现意外参数"),
                ("invalid value", "无效的取值"),
                (" for '", "，参数 '"),
                (" as a value, use ", " 作为值传入，请使用 "),
                ("possible values:", "可选值："),
                ("tip: to pass", "提示：若要将"),
                ("Usage:", "用法:"),
                ("error:", "错误:"),
            ] {
                localized = localized.replace(from, to);
            }
            localize_help_hint_zh(localized)
        }
        Language::EnUs => message.to_owned(),
    }
}

fn localize_help_hint_zh(message: String) -> String {
    let exact = "For more information, try '--help'.";
    if message.contains(exact) {
        return message.replace(exact, "如需更多信息，请使用 '--help'。");
    }

    let prefix = "For more information, try '";
    let Some(start) = message.find(prefix) else {
        return message;
    };

    let mut localized = String::with_capacity(message.len() + 16);
    localized.push_str(&message[..start]);
    localized.push_str("如需更多信息，请使用 '");

    let rest = &message[start + prefix.len()..];
    if let Some(end) = rest.find("'.") {
        localized.push_str(&rest[..end]);
        localized.push_str("'。");
        localized.push_str(&rest[end + 2..]);
    } else {
        localized.push_str(rest);
    }

    localized
}

fn catalog_for(lang: Language) -> &'static LocaleCatalog {
    match lang {
        Language::ZhCn => catalog_by_code("zh-CN"),
        Language::EnUs => catalog_by_code("en-US"),
    }
    .or_else(|| catalog_by_code(DEFAULT_LOCALE.code()))
    .expect("at least one CLI locale catalog must exist")
}

fn catalog_by_code(code: &str) -> Option<&'static LocaleCatalog> {
    catalogs().iter().find(|catalog| catalog.meta.code == code)
}

fn catalogs() -> &'static [LocaleCatalog] {
    static CATALOGS: OnceLock<Vec<LocaleCatalog>> = OnceLock::new();
    CATALOGS.get_or_init(load_catalogs)
}

fn load_catalogs() -> Vec<LocaleCatalog> {
    let catalogs = EMBEDDED_CATALOGS
        .iter()
        .map(|raw| {
            serde_json::from_str::<LocaleCatalog>(raw).expect("embedded CLI locale JSON must parse")
        })
        .collect::<Vec<_>>();

    validate_catalogs(&catalogs);
    catalogs
}

fn validate_catalogs(catalogs: &[LocaleCatalog]) {
    assert!(
        !catalogs.is_empty(),
        "at least one CLI locale catalog is required"
    );

    let mut seen_codes = BTreeSet::new();
    for catalog in catalogs {
        assert!(
            seen_codes.insert(catalog.meta.code.clone()),
            "duplicate CLI locale code: {}",
            catalog.meta.code
        );
        assert!(
            !catalog.meta.aliases.is_empty(),
            "CLI locale {} must declare at least one alias",
            catalog.meta.code
        );
    }

    assert!(
        catalogs
            .iter()
            .any(|catalog| normalize_language_tag(&catalog.meta.code)
                == normalize_language_tag(DEFAULT_LOCALE.code())),
        "default CLI locale must exist"
    );

    let expected_commands = catalogs[0]
        .commands
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_args = catalogs[0].args.keys().cloned().collect::<BTreeSet<_>>();
    let expected_command_args = catalogs[0]
        .command_args
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();

    for catalog in catalogs.iter().skip(1) {
        let command_keys = catalog.commands.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(
            command_keys, expected_commands,
            "CLI locale {} has mismatched command translation keys",
            catalog.meta.code
        );

        let arg_keys = catalog.args.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(
            arg_keys, expected_args,
            "CLI locale {} has mismatched argument translation keys",
            catalog.meta.code
        );

        let command_arg_keys = catalog
            .command_args
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            command_arg_keys, expected_command_args,
            "CLI locale {} has mismatched command argument translation keys",
            catalog.meta.code
        );
    }
}

fn leak_str(value: &str) -> &'static str {
    Box::leak(value.to_owned().into_boxed_str())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocaleCatalog {
    meta: LocaleMeta,
    headings: LocaleHeadings,
    help: LocaleHelp,
    commands: BTreeMap<String, CommandText>,
    args: BTreeMap<String, String>,
    #[serde(default)]
    command_args: BTreeMap<String, String>,
}

impl LocaleCatalog {
    fn command_about(&self, key: &str) -> &str {
        self.commands
            .get(key)
            .unwrap_or_else(|| {
                panic!(
                    "missing CLI command translation key `{key}` for locale {}",
                    self.meta.code
                )
            })
            .about
            .as_str()
    }

    fn arg_ids(&self) -> impl Iterator<Item = &str> {
        self.args.keys().map(String::as_str)
    }

    fn arg_help(&self, command_key: &str, arg_id: &str) -> &str {
        self.command_args
            .get(&format!("{command_key}.{arg_id}"))
            .or_else(|| self.args.get(arg_id))
            .unwrap_or_else(|| {
                panic!(
                    "missing CLI argument translation key `{arg_id}` for locale {}",
                    self.meta.code
                )
            })
            .as_str()
    }

    fn has_command_arg_override(&self, command_key: &str, arg_id: &str) -> bool {
        self.command_args
            .contains_key(&format!("{command_key}.{arg_id}"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocaleMeta {
    code: String,
    aliases: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocaleHeadings {
    usage: String,
    global_options: String,
    options: String,
    arguments: String,
    commands: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocaleHelp {
    root_after_help: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandText {
    about: String,
}

struct VersionLabels {
    version: &'static str,
    rustc: &'static str,
    target: &'static str,
}

//! Command implementations

mod auth;
mod download;
mod forward;
mod hello;
mod upload;
mod version;

use crate::cli::{AuthCommands, Commands, LoginCommands};
use anyhow::Result;

/// Execute a CLI command
pub async fn execute(command: Commands) -> Result<()> {
    match command {
        Commands::Hello { name } => hello::run(&name),
        Commands::Version => version::run(),
        Commands::Auth(cmd) => execute_auth(cmd).await,
        Commands::Upload(args) => {
            upload::run(
                args.path,
                args.chat,
                args.include,
                args.exclude,
                args.rm,
                args.topic,
                args.account,
                args.all_accounts,
                args.caption,
                args.to,
                args.group,
            )
            .await
        }
        Commands::Download(args) => {
            download::run(
                args.url,
                args.path,
                args.include,
                args.exclude,
                args.template,
                args.account,
            )
            .await
        }
        Commands::Forward(args) => {
            forward::run(
                args.from,
                args.from_chat,
                args.to,
                args.mode,
                args.topic,
                args.account,
                args.drop_author,
            )
            .await
        }
    }
}

async fn execute_auth(cmd: AuthCommands) -> Result<()> {
    match cmd {
        AuthCommands::Login(login_cmd) => match login_cmd {
            LoginCommands::Add { name, method } => auth::login::add(name, method).await,
            LoginCommands::List => auth::login::list(),
            LoginCommands::Remove { id } => auth::login::remove(id),
            LoginCommands::Use { id } => auth::login::use_account(id),
        },
        AuthCommands::Logout { id, all } => auth::logout(id, all),
        AuthCommands::Status => auth::status().await,
    }
}

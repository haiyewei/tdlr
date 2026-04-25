//! Auth command arguments

use clap::{Subcommand, ValueEnum};

#[derive(Debug, Subcommand)]
pub enum AuthCommands {
    /// Login - manage Telegram accounts
    #[command(subcommand)]
    Login(LoginCommands),
    /// Logout from account(s)
    Logout {
        /// User ID to logout (logout from active account if not specified)
        #[arg(short, long)]
        id: Option<i64>,
        /// Logout from all accounts
        #[arg(long)]
        all: bool,
    },
    /// Show status of all accounts (concurrent check)
    Status,
}

#[derive(Debug, Subcommand)]
pub enum LoginCommands {
    /// Add a new Telegram account
    Add {
        /// Account name/alias (optional, for display only)
        #[arg(short, long)]
        name: Option<String>,
        /// Login method: phone or qr
        #[arg(short, long, value_enum, default_value = "qr")]
        method: LoginMethod,
        /// Preferred delivery channel for phone login codes
        #[arg(long, value_enum, default_value = "auto")]
        code_via: LoginCodeVia,
    },
    /// List all logged in accounts
    List,
    /// Remove an account by user ID
    Remove {
        /// User ID to remove
        id: i64,
    },
    /// Switch active account by user ID
    Use {
        /// User ID to use
        id: i64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum LoginMethod {
    /// Login with phone number and verification code
    Phone,
    /// Login by scanning QR code with Telegram app
    Qr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LoginCodeVia {
    /// Let Telegram choose the delivery channel
    Auto,
    /// Prefer receiving the code inside the Telegram app
    App,
    /// Prefer receiving the code over SMS
    Sms,
}

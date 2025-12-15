//! Authentication commands

pub mod login;
mod logout;
mod status;

pub use logout::run as logout;
pub use status::run as status;

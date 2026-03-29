//! Authentication methods

mod phone;
mod qrcode;

pub use phone::login_with_phone;
pub use qrcode::login_with_qrcode;
pub(crate) use qrcode::{export_qr_login_token, try_import_login, QrLoginTokenExport};

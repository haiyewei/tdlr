//! Authentication methods

mod phone;
mod qrcode;

pub use phone::login_with_phone;
pub use qrcode::login_with_qrcode;

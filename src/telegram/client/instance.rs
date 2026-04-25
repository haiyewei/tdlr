//! Single Telegram client instance

use crate::telegram::session::SessionManager;
use anyhow::Result;
use grammers_client::{client::PasswordToken, peer::User, Client, SignInError};
use grammers_mtsender::{ConnectionParams, InvocationError, SenderPool};
use grammers_session::storages::SqliteSession;
use grammers_session::types::{PeerInfo, UpdateState, UpdatesState};
use grammers_session::Session;
use grammers_tl_types as tl;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// App version from Cargo.toml
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginCodePreference {
    Auto,
    App,
    Sms,
}

impl LoginCodePreference {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::App => "app",
            Self::Sms => "sms",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginCodeDelivery {
    App,
    Sms,
    Call,
    FlashCall,
    MissedCall,
    FragmentSms,
    FirebaseSms,
    EmailCode,
    SetUpEmailRequired,
    SmsWord,
    SmsPhrase,
}

impl LoginCodeDelivery {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Sms => "sms",
            Self::Call => "call",
            Self::FlashCall => "flash-call",
            Self::MissedCall => "missed-call",
            Self::FragmentSms => "fragment-sms",
            Self::FirebaseSms => "firebase-sms",
            Self::EmailCode => "email-code",
            Self::SetUpEmailRequired => "setup-email-required",
            Self::SmsWord => "sms-word",
            Self::SmsPhrase => "sms-phrase",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhoneLoginCodeState {
    pub phone: String,
    pub phone_code_hash: String,
    pub sent_via: LoginCodeDelivery,
    pub next_via: Option<LoginCodeDelivery>,
    pub timeout: Option<i32>,
}

fn map_sent_code_type(sent_type: &tl::enums::auth::SentCodeType) -> LoginCodeDelivery {
    match sent_type {
        tl::enums::auth::SentCodeType::App(_) => LoginCodeDelivery::App,
        tl::enums::auth::SentCodeType::Sms(_) => LoginCodeDelivery::Sms,
        tl::enums::auth::SentCodeType::Call(_) => LoginCodeDelivery::Call,
        tl::enums::auth::SentCodeType::FlashCall(_) => LoginCodeDelivery::FlashCall,
        tl::enums::auth::SentCodeType::MissedCall(_) => LoginCodeDelivery::MissedCall,
        tl::enums::auth::SentCodeType::EmailCode(_) => LoginCodeDelivery::EmailCode,
        tl::enums::auth::SentCodeType::SetUpEmailRequired(_) => {
            LoginCodeDelivery::SetUpEmailRequired
        }
        tl::enums::auth::SentCodeType::FragmentSms(_) => LoginCodeDelivery::FragmentSms,
        tl::enums::auth::SentCodeType::FirebaseSms(_) => LoginCodeDelivery::FirebaseSms,
        tl::enums::auth::SentCodeType::SmsWord(_) => LoginCodeDelivery::SmsWord,
        tl::enums::auth::SentCodeType::SmsPhrase(_) => LoginCodeDelivery::SmsPhrase,
    }
}

fn map_code_type(code_type: &tl::enums::auth::CodeType) -> LoginCodeDelivery {
    match code_type {
        tl::enums::auth::CodeType::Sms => LoginCodeDelivery::Sms,
        tl::enums::auth::CodeType::Call => LoginCodeDelivery::Call,
        tl::enums::auth::CodeType::FlashCall => LoginCodeDelivery::FlashCall,
        tl::enums::auth::CodeType::MissedCall => LoginCodeDelivery::MissedCall,
        tl::enums::auth::CodeType::FragmentSms => LoginCodeDelivery::FragmentSms,
    }
}

fn build_phone_login_state(
    phone: &str,
    sent_code: tl::types::auth::SentCode,
) -> PhoneLoginCodeState {
    PhoneLoginCodeState {
        phone: phone.to_string(),
        phone_code_hash: sent_code.phone_code_hash,
        sent_via: map_sent_code_type(&sent_code.r#type),
        next_via: sent_code.next_type.as_ref().map(map_code_type),
        timeout: sent_code.timeout,
    }
}

/// Create connection params with custom app info
fn connection_params() -> ConnectionParams {
    ConnectionParams {
        app_version: APP_VERSION.to_string(),
        device_model: "Desktop".to_string(),
        ..ConnectionParams::default()
    }
}

/// Single Telegram client instance
pub struct TelegramClient {
    pub client: Client,
    pub user_id: i64,
    api_id: i32,
    session: Arc<SqliteSession>,
    network_handle: JoinHandle<()>,
}

impl TelegramClient {
    /// Create a new client for the given user_id
    pub async fn new(user_id: i64, api_id: i32) -> Result<Self> {
        SessionManager::ensure_dir()?;

        let session_path = SessionManager::session_path(user_id);
        let session = Arc::new(SqliteSession::open(session_path.to_str().unwrap()).await?);
        let pool =
            SenderPool::with_configuration(Arc::clone(&session), api_id, connection_params());
        let client = Client::new(pool.handle.clone());

        let network_handle = {
            let runner = pool.runner;
            tokio::spawn(async move {
                runner.run().await;
            })
        };

        Ok(Self {
            client,
            user_id,
            api_id,
            session,
            network_handle,
        })
    }

    /// Create a new client with a temp session name (for login)
    pub async fn new_temp(temp_name: &str, api_id: i32) -> Result<Self> {
        SessionManager::ensure_dir()?;

        let session_path = SessionManager::session_path_str(temp_name);
        let session = Arc::new(SqliteSession::open(session_path.to_str().unwrap()).await?);
        let pool =
            SenderPool::with_configuration(Arc::clone(&session), api_id, connection_params());
        let client = Client::new(pool.handle.clone());

        let network_handle = {
            let runner = pool.runner;
            tokio::spawn(async move {
                runner.run().await;
            })
        };

        Ok(Self {
            client,
            user_id: 0, // Will be set after login
            api_id,
            session,
            network_handle,
        })
    }

    /// Get reference to the underlying client
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Check if authorized
    pub async fn is_authorized(&self) -> Result<bool> {
        Ok(self.client.is_authorized().await?)
    }

    /// Get current user
    pub async fn get_me(&self) -> Result<grammers_client::peer::User> {
        Ok(self.client.get_me().await?)
    }

    /// Get current home DC ID
    pub fn home_dc_id(&self) -> i32 {
        self.session.home_dc_id()
    }

    /// Get a peer reference from the session cache
    pub async fn get_peer_ref(
        &self,
        peer_id: grammers_session::types::PeerId,
    ) -> Option<grammers_session::types::PeerRef> {
        self.session.peer_ref(peer_id).await
    }

    /// Set home DC ID (needed after DC migration during login)
    pub async fn set_home_dc_id(&self, dc_id: i32) {
        self.session.set_home_dc_id(dc_id).await;
    }

    async fn invoke_auth<R: tl::RemoteCall>(
        &self,
        request: &R,
    ) -> Result<R::Return, InvocationError> {
        match self.client.invoke(request).await {
            Ok(result) => Ok(result),
            Err(InvocationError::Rpc(err)) if err.code == 303 => {
                let Some(new_dc_id) = err.value.map(|value| value as i32) else {
                    return Err(InvocationError::Rpc(err));
                };
                self.session.set_home_dc_id(new_dc_id).await;
                self.client.invoke_in_dc(new_dc_id, request).await
            }
            Err(err) => Err(err),
        }
    }

    async fn complete_authorization(
        &self,
        auth: tl::types::auth::Authorization,
    ) -> Result<User, InvocationError> {
        let update_state = self
            .client
            .invoke(&tl::functions::updates::GetState {})
            .await
            .ok();

        let user = User::from_raw(&self.client, auth.user);
        let auth = user.to_ref().await.unwrap().auth;

        self.session
            .cache_peer(&PeerInfo::User {
                id: user.id().bare_id(),
                auth: Some(auth),
                bot: Some(user.is_bot()),
                is_self: Some(true),
            })
            .await;

        if let Some(tl::enums::updates::State::State(state)) = update_state {
            self.session
                .set_update_state(UpdateState::All(UpdatesState {
                    pts: state.pts,
                    qts: state.qts,
                    date: state.date,
                    seq: state.seq,
                    channels: Vec::new(),
                }))
                .await;
        }

        Ok(user)
    }

    async fn get_password_information(&self) -> Result<PasswordToken, InvocationError> {
        let password: tl::types::account::Password = self
            .client
            .invoke(&tl::functions::account::GetPassword {})
            .await?
            .into();
        Ok(PasswordToken::new(password))
    }

    pub async fn send_login_code(
        &self,
        phone: &str,
        api_hash: &str,
    ) -> Result<PhoneLoginCodeState, InvocationError> {
        let request = tl::functions::auth::SendCode {
            phone_number: phone.to_string(),
            api_id: self.api_id,
            api_hash: api_hash.to_string(),
            settings: tl::types::CodeSettings {
                allow_flashcall: false,
                current_number: false,
                allow_app_hash: false,
                allow_missed_call: false,
                allow_firebase: false,
                logout_tokens: None,
                token: None,
                app_sandbox: None,
                unknown_number: false,
            }
            .into(),
        };

        match self.invoke_auth(&request).await? {
            tl::enums::auth::SentCode::Code(code) => Ok(build_phone_login_state(phone, code)),
            tl::enums::auth::SentCode::Success(_) => {
                panic!("should not have logged in yet")
            }
            tl::enums::auth::SentCode::PaymentRequired(_) => unimplemented!(),
        }
    }

    pub async fn resend_login_code(
        &self,
        state: &PhoneLoginCodeState,
    ) -> Result<PhoneLoginCodeState, InvocationError> {
        let request = tl::functions::auth::ResendCode {
            phone_number: state.phone.clone(),
            phone_code_hash: state.phone_code_hash.clone(),
            reason: None,
        };

        match self.invoke_auth(&request).await? {
            tl::enums::auth::SentCode::Code(code) => {
                Ok(build_phone_login_state(&state.phone, code))
            }
            tl::enums::auth::SentCode::Success(_) => {
                panic!("should not have logged in yet")
            }
            tl::enums::auth::SentCode::PaymentRequired(_) => unimplemented!(),
        }
    }

    pub async fn sign_in_with_phone_code(
        &self,
        state: &PhoneLoginCodeState,
        code: &str,
    ) -> Result<User, SignInError> {
        match self
            .invoke_auth(&tl::functions::auth::SignIn {
                phone_number: state.phone.clone(),
                phone_code_hash: state.phone_code_hash.clone(),
                phone_code: Some(code.to_string()),
                email_verification: None,
            })
            .await
        {
            Ok(tl::enums::auth::Authorization::Authorization(auth)) => self
                .complete_authorization(auth)
                .await
                .map_err(SignInError::Other),
            Ok(tl::enums::auth::Authorization::SignUpRequired(_)) => {
                Err(SignInError::SignUpRequired)
            }
            Err(err) if err.is("SESSION_PASSWORD_NEEDED") => {
                match self.get_password_information().await {
                    Ok(token) => Err(SignInError::PasswordRequired(token)),
                    Err(err) => Err(SignInError::Other(err)),
                }
            }
            Err(err) if err.is("PHONE_CODE_*") => Err(SignInError::InvalidCode),
            Err(err) => Err(SignInError::Other(err)),
        }
    }
}

impl Drop for TelegramClient {
    fn drop(&mut self) {
        self.network_handle.abort();
    }
}

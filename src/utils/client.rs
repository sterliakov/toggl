use std::ops::{Deref, DerefMut};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client as ReqwestClient;
pub use reqwest::Result;

// use surf::middleware::{Middleware, Next};
// pub use surf::Result;
// use surf::{Request, Response};

pub struct Client {
    client: ReqwestClient,
}

impl Client {
    pub const BASE_URL: &'static str = "https://api.track.toggl.com";

    pub fn from_email_password(email: &str, password: &str) -> Self {
        Self {
            client: ReqwestClient::builder()
                .timeout(Duration::from_secs(10))
                .default_headers(Self::make_headers(email, password))
                .build()
                .expect("Invalid client"),
        }
    }

    pub fn from_api_token(api_token: &str) -> Self {
        Self::from_email_password(api_token, "api_token")
    }

    fn make_headers(email: &str, password: &str) -> HeaderMap {
        let auth_encoded =
            STANDARD.encode(format!("{email}:{password}").into_bytes());
        [
            (
                HeaderName::from_static("Content-Type"),
                HeaderValue::from_static("application/json"),
            ),
            (
                HeaderName::from_static("Authorization"),
                HeaderValue::from_str(format!("Basic {auth_encoded}").as_str())
                    .expect("Invalid auth header"),
            ),
        ]
        .into_iter()
        .collect()
    }
}

impl Deref for Client {
    type Target = ReqwestClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

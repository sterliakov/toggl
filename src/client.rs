use base64::{engine::general_purpose::STANDARD, Engine as _};
use log::{error, info};
use std::ops::{Deref, DerefMut};
use surf::middleware::{Middleware, Next};
pub use surf::Result;
use surf::{Request, Response};

pub struct Client {
    client: surf::Client,
}

impl Client {
    pub const BASE_URL: &'static str = "https://api.track.toggl.com";

    pub fn from_email_password(email: &str, password: &str) -> Self {
        Self {
            client: surf::Client::new()
                .with(AuthMiddleware(email.to_string(), password.to_string())),
        }
    }

    pub fn from_api_token(api_token: &str) -> Self {
        Self::from_email_password(api_token, "api_token")
    }

    pub async fn check_status(res: &mut surf::Response) -> Result<()> {
        let status = res.status();
        if !status.is_success() {
            let binary = &res.body_bytes().await?;
            let msg = if binary.is_empty() {
                error!("Received an unsuccessful response (empty body).");
                status.to_string()
            } else {
                let response_text = std::str::from_utf8(binary)?.to_string();
                error!("Received an unsuccessful response (non-empty body: '{response_text}').");
                response_text
            };
            Err(surf::Error::from_str(status, msg))
        } else {
            info!("Received a successful response.");
            Ok(())
        }
    }
}

impl Deref for Client {
    type Target = surf::Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

/// Add Auth and Content-Type headers to all requests
pub struct AuthMiddleware(String, String);

#[surf::utils::async_trait]
impl Middleware for AuthMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        client: surf::Client,
        next: Next<'_>,
    ) -> Result<Response> {
        req.set_header("Content-Type", "application/json");
        let auth_encoded =
            STANDARD.encode(format!("{}:{}", self.0, self.1).into_bytes());
        req.set_header(
            "Authorization",
            format!("Basic {auth_encoded}").as_str(),
        );
        next.run(req, client).await
    }
}

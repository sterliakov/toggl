use iced::widget::{button, column, container, scrollable, text_input};
use iced::{Element, Fill, Task as Command};
use serde::{Deserialize, Serialize};

use crate::client::{Client, Result as NetResult};

#[derive(Clone, Debug, Default)]
pub struct LoginScreen {
    email: String,
    password: String,
}

#[derive(Clone, Debug)]
pub enum LoginScreenMessage {
    EmailEdited(String),
    PasswordEdited(String),
    Submit,
    Completed(Result<String, String>),
}

impl LoginScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view(&self) -> Element<LoginScreenMessage> {
        let content = column![
            text_input("Email", &self.email)
                .id("email-input")
                .on_input(LoginScreenMessage::EmailEdited),
            text_input("Password", &self.password)
                .id("password-input")
                .on_input(LoginScreenMessage::PasswordEdited),
            button("Login")
                .on_press(LoginScreenMessage::Submit)
                .style(button::primary)
        ]
        .spacing(10);

        scrollable(container(content).center_x(Fill).padding(40)).into()
    }

    pub fn update(&mut self, message: LoginScreenMessage) -> Command<LoginScreenMessage> {
        match &message {
            LoginScreenMessage::EmailEdited(email) => self.email = email.clone(),
            LoginScreenMessage::PasswordEdited(password) => self.password = password.clone(),
            LoginScreenMessage::Submit => {
                return Command::future(Self::submit(self.email.clone(), self.password.clone()));
            }
            LoginScreenMessage::Completed(_) => {}
        };
        Command::none()
    }

    async fn submit(email: String, password: String) -> LoginScreenMessage {
        LoginScreenMessage::Completed(
            Self::call_submit(&email, &password)
                .await
                .map_err(|e| e.to_string()),
        )
    }

    async fn call_submit(email: &str, password: &str) -> NetResult<String> {
        let client = Client::from_email_password(&email, &password);
        let data = client
            .get([Client::BASE_URL, "/api/v9/me"].join(""))
            .send()
            .await?
            .body_json::<LoginResponse>()
            .await?;
        Ok(data.api_token)
    }
}

#[derive(Serialize, Deserialize)]
struct LoginResponse {
    api_token: String,
}

#[cfg(test)]
mod test {
    use super::LoginScreen;

    #[async_std::test]
    async fn test_load() {
        let token = LoginScreen::call_submit(
            &std::env::var("TEST_EMAIL").expect("Please pass TEST_EMAIL"),
            &std::env::var("TEST_PASSWORD").expect("Please pass TEST_PASSWORD"),
        )
        .await
        .expect("Must not fail");
        assert_ne!(token.len(), 0);
    }
}

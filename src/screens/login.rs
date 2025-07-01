use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Fill, Task as Command};
use serde::{Deserialize, Serialize};

use crate::state::State;
use crate::utils::{Client, NetResult};
use crate::widgets::CustomWidget;

#[derive(Clone, Debug, Default)]
pub struct LoginScreen {
    email: String,
    password: String,
    error: String,
}

#[derive(Clone, Debug)]
pub enum LoginScreenMessage {
    EmailEdited(String),
    PasswordEdited(String),
    Submit,
    Completed(String),
    Error(String),
}

impl CustomWidget<LoginScreenMessage> for LoginScreen {
    fn view(&self, _state: &State) -> Element<'_, LoginScreenMessage> {
        let content = column![
            text_input("Email", &self.email)
                .id("email-input")
                .on_submit(LoginScreenMessage::Submit)
                .on_input(LoginScreenMessage::EmailEdited),
            text_input("Password", &self.password)
                .id("password-input")
                .secure(true)
                .on_submit(LoginScreenMessage::Submit)
                .on_input(LoginScreenMessage::PasswordEdited),
            button("Login")
                .on_press(LoginScreenMessage::Submit)
                .style(button::primary),
            text(&self.error).style(text::danger)
        ]
        .spacing(10);

        scrollable(container(content).center_x(Fill).padding(40)).into()
    }

    fn update(
        &mut self,
        message: LoginScreenMessage,
        _state: &State,
    ) -> Command<LoginScreenMessage> {
        match message {
            LoginScreenMessage::EmailEdited(email) => self.email = email,
            LoginScreenMessage::PasswordEdited(password) => {
                self.password = password;
            }
            LoginScreenMessage::Error(err) => self.error = err,
            LoginScreenMessage::Submit => {
                return Command::future(self.clone().submit());
            }
            LoginScreenMessage::Completed(_) => {}
        }
        Command::none()
    }
}

impl LoginScreen {
    pub fn new() -> Self {
        Self::default()
    }

    async fn submit(self) -> LoginScreenMessage {
        if self.email.is_empty() {
            return LoginScreenMessage::Error(
                "Email must not be empty".to_owned(),
            );
        }
        if self.password.is_empty() {
            return LoginScreenMessage::Error(
                "Password must not be empty".to_owned(),
            );
        }
        match Self::call_submit(&self.email, &self.password).await {
            Ok(token) => LoginScreenMessage::Completed(token),
            Err(e) => LoginScreenMessage::Error(e.to_string()),
        }
    }

    async fn call_submit(email: &str, password: &str) -> NetResult<String> {
        let client = Client::from_email_password(email, password);
        let mut rsp = client
            .get(format!("{}/api/v9/me", Client::BASE_URL))
            .send()
            .await?;
        Client::check_status(&mut rsp).await?;
        Ok(rsp.body_json::<LoginResponse>().await?.api_token)
    }
}

#[derive(Serialize, Deserialize)]
struct LoginResponse {
    api_token: String,
}

#[cfg(test)]
mod test {
    use super::LoginScreen;
    use crate::test;

    #[async_std::test]
    async fn test_load() {
        let token = LoginScreen::call_submit(
            &test::test_email(),
            &test::test_password(),
        )
        .await
        .expect("Must not fail");
        assert_ne!(token.len(), 0);
    }
}

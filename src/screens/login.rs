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
    profile_name: Option<String>,
    error: String,
}

#[derive(Clone, Debug)]
pub enum LoginScreenMessage {
    EmailEdited(String),
    PasswordEdited(String),
    ProfileNameEdited(String),
    Submit,
    Completed { email: String, api_token: String },
    Error(String),
}

impl CustomWidget<LoginScreenMessage> for LoginScreen {
    fn view(&self, _state: &State) -> Element<'_, LoginScreenMessage> {
        use LoginScreenMessage::*;

        let content = column![
            text_input("Email *", &self.email)
                .id("email-input")
                .on_submit(Submit)
                .on_input(EmailEdited),
            text_input("Password *", &self.password)
                .id("password-input")
                .secure(true)
                .on_submit(Submit)
                .on_input(PasswordEdited),
            text_input(
                "Profile name",
                self.profile_name.as_ref().unwrap_or(&self.email)
            )
            .id("profile-name-input")
            .on_submit(Submit)
            .on_input(ProfileNameEdited),
            button("Login").on_press(Submit).style(button::primary),
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
        use LoginScreenMessage::*;

        match message {
            EmailEdited(email) => self.email = email,
            PasswordEdited(password) => {
                self.password = password;
            }
            ProfileNameEdited(profile_name) => {
                self.profile_name = Some(profile_name);
            }
            Error(err) => self.error = err,
            Submit => {
                return Command::future(self.clone().submit());
            }
            Completed { .. } => {}
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
        let profile_name = self
            .profile_name
            .clone()
            .unwrap_or_else(|| self.email.clone());
        if self.profile_name.is_some_and(|p| p.is_empty()) {
            return LoginScreenMessage::Error(
                "Profile name must not be empty".to_owned(),
            );
        }
        match Self::call_submit(&self.email, &self.password).await {
            Ok(token) => LoginScreenMessage::Completed {
                email: profile_name,
                api_token: token,
            },
            Err(e) => LoginScreenMessage::Error(e.to_string()),
        }
    }

    async fn call_submit(email: &str, password: &str) -> NetResult<String> {
        let client = Client::from_email_password(email, password);
        let rsp: LoginResponse = client
            .get(format!("{}/api/v9/me", Client::BASE_URL))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(rsp.api_token)
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

    #[tokio::test]
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

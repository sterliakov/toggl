// use iced::keyboard;
use iced::widget::{
    center, column, container, scrollable, text
};
// use iced::window;
use iced::{Center, Element, Fill, Task as Command};

use serde::{Deserialize, Serialize};

mod client;
mod login;
mod time_entry;

use crate::client::{Client, Result as NetResult};
use crate::login::{LoginScreen, LoginScreenMessage};
use crate::time_entry::TimeEntry;

pub fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        // .subscription(App::subscription)
        .window_size((500.0, 800.0))
        .run_with(App::new)
}

#[derive(Clone, Debug)]
enum App {
    Loading,
    Unauthed(LoginScreen),
    Authed,
    Loaded(State),
}

#[derive(Clone, Debug, Default)]
struct State {
    api_token: String,
    time_entries: Vec<TimeEntry>,
}

#[derive(Debug)]
enum Message {
    Loaded(Result<SavedState, LoadError>),
    DataFetched(NetResult<State>),
    LoginMessage(LoginScreenMessage),
    Discarded,
}

impl App {
    fn new() -> (Self, Command<Message>) {
        (
            Self::Loading,
            Command::perform(SavedState::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        "Toggl Tracker".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            App::Loading => match message {
                Message::Loaded(Ok(state)) => {
                    *self = App::Authed;
                    Command::future(App::load_everything(state.api_token))
                }
                Message::Loaded(_) => {
                    *self = App::Unauthed(LoginScreen::new());
                    Command::none()
                }
                _ => Command::none(),
            },
            App::Unauthed(screen) => match message {
                Message::LoginMessage(LoginScreenMessage::Completed(Ok(api_token))) => {
                    *self = App::Authed;
                    Command::batch(vec![
                        Command::future(
                            SavedState {
                                api_token: api_token.clone(),
                            }
                            .save(),
                        )
                        .map(|_| Message::Discarded),
                        Command::future(App::load_everything(api_token)),
                    ])
                }
                Message::LoginMessage(msg) => {
                    screen.update(msg).map(|msg| Message::LoginMessage(msg))
                }
                _ => Command::none(),
            },
            App::Authed => match message {
                Message::DataFetched(Ok(state)) => {
                    *self = App::Loaded(state);
                    Command::none()
                }
                _ => Command::none(),
            },
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        match self {
            App::Loading => loading_message(),
            App::Authed => loading_message(),
            App::Unauthed(screen) => screen.view().map(move |msg| Message::LoginMessage(msg)),
            App::Loaded(State { time_entries, .. }) => {
                let content = column(
                    time_entries
                        .iter()
                        .map(|task| task.view().map(|_| Message::Discarded)),
                )
                .spacing(10);

                scrollable(container(content).center_x(Fill).padding(40)).into()
            }
        }
    }

    async fn load_everything(api_token: String) -> Message {
        let client = Client::from_api_token(&api_token);
        TimeEntry::load(&client)
            .await
            .map(|time_entries| {
                Message::DataFetched(NetResult::Ok(State {
                    time_entries,
                    api_token,
                }))
            })
            .unwrap_or_else(|e| Message::DataFetched(NetResult::Err(e)))
    }

    // fn subscription(&self) -> Subscription<Message> {
    //     use keyboard::key;

    //     keyboard::on_key_press(|key, modifiers| {
    //         let keyboard::Key::Named(key) = key else {
    //             return None;
    //         };

    //         match (key, modifiers) {
    //             (key::Named::Tab, _) => Some(Message::TabPressed {
    //                 shift: modifiers.shift(),
    //             }),
    //             (key::Named::ArrowUp, keyboard::Modifiers::SHIFT) => {
    //                 Some(Message::ToggleFullscreen(window::Mode::Fullscreen))
    //             }
    //             (key::Named::ArrowDown, keyboard::Modifiers::SHIFT) => {
    //                 Some(Message::ToggleFullscreen(window::Mode::Windowed))
    //             }
    //             _ => None,
    //         }
    //     })
    // }
}

fn loading_message<'a>() -> Element<'a, Message> {
    center(text("Loading...").width(Fill).align_x(Center).size(50)).into()
}

// Persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedState {
    api_token: String,
}

#[derive(Debug, Clone)]
enum LoadError {
    File,
    Format,
}

#[derive(Debug, Clone)]
enum SaveError {
    File,
    Write,
    Format,
}

impl SavedState {
    fn path() -> std::path::PathBuf {
        let mut path = if let Some(project_dirs) =
            directories_next::ProjectDirs::from("rs", "Iced", "toggl-tracker")
        {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        };

        path.push("toggl.json");
        path
    }

    async fn load() -> Result<SavedState, LoadError> {
        use async_std::prelude::*;

        let mut contents = String::new();

        let mut file = async_std::fs::File::open(Self::path())
            .await
            .map_err(|_| LoadError::File)?;

        file.read_to_string(&mut contents)
            .await
            .map_err(|_| LoadError::File)?;

        serde_json::from_str(&contents).map_err(|_| LoadError::Format)
    }

    async fn save(self) -> Result<(), SaveError> {
        use async_std::prelude::*;

        let json = serde_json::to_string_pretty(&self).map_err(|_| SaveError::Format)?;

        let path = Self::path();

        if let Some(dir) = path.parent() {
            async_std::fs::create_dir_all(dir)
                .await
                .map_err(|_| SaveError::File)?;
        }

        {
            let mut file = async_std::fs::File::create(path)
                .await
                .map_err(|_| SaveError::File)?;

            file.write_all(json.as_bytes())
                .await
                .map_err(|_| SaveError::Write)?;
        }

        // This is a simple way to save at most once every couple seconds
        async_std::task::sleep(std::time::Duration::from_secs(2)).await;

        Ok(())
    }
}

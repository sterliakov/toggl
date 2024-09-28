use iced::widget::{
    center, column, container, horizontal_rule, row, scrollable, text,
    text_input,
};
use iced::window;
use iced::{Center, Element, Fill, Padding, Task as Command};
use itertools::Itertools;
use lazy_static::lazy_static;

use serde::{Deserialize, Serialize};

mod client;
mod edit_time_entry;
mod login;
mod project;
mod related_info;
mod time_entry;
mod utils;
mod workspace;

use crate::client::Client;
use crate::edit_time_entry::{EditTimeEntry, EditTimeEntryMessage};
use crate::login::{LoginScreen, LoginScreenMessage};
use crate::project::Project;
use crate::related_info::ExtendedMe;
use crate::time_entry::CreateTimeEntry;
use crate::time_entry::{TimeEntry, TimeEntryMessage};
use crate::utils::date_as_human_readable;
use crate::workspace::Workspace;

pub fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .window_size((500.0, 600.0))
        .run_with(App::new)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct State {
    api_token: String,
    time_entries: Vec<TimeEntry>,
    running_entry: Option<TimeEntry>,
    projects: Vec<Project>,
    workspaces: Vec<Workspace>,
}

#[derive(Debug, Default)]
struct TemporaryState {
    new_running_entry_description: String,
}

#[derive(Debug, Default)]
struct App {
    state: State,
    screen: Screen,
    window_id: Option<window::Id>,
}

#[derive(Debug, Default)]
enum Screen {
    #[default]
    Loading,
    Unauthed(LoginScreen),
    Authed,
    Loaded(TemporaryState),
    EditEntry(EditTimeEntry),
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<State, LoadError>),
    DataFetched(Result<State, String>),
    LoginProxy(LoginScreenMessage),
    TimeEntryProxy(TimeEntryMessage),
    EditTimeEntryProxy(EditTimeEntryMessage),
    SetInitialRunningEntry(String),
    SubmitNewRunningEntry,
    Tick,
    Reload,
    Discarded,
    WindowIdReceived(Option<window::Id>),
}

lazy_static! {
    static ref RUNNING_ICON: window::Icon = window::icon::from_file_data(
        include_bytes!("../assets/icon.png"),
        None
    )
    .expect("Icon must parse");
    static ref DEFAULT_ICON: window::Icon = window::icon::from_file_data(
        include_bytes!("../assets/icon-gray.png"),
        None
    )
    .expect("Icon must parse");
}

impl App {
    fn new() -> (Self, Command<Message>) {
        (
            Self {
                state: State::default(),
                screen: Screen::Loading,
                window_id: None,
            },
            Command::batch(vec![
                Command::perform(State::load(), Message::Loaded),
                iced::window::get_latest().map(Message::WindowIdReceived),
            ]),
        )
    }

    fn title(&self) -> String {
        if self.state.running_entry.is_some() {
            "* Toggl Tracker".to_string()
        } else {
            "Toggl Tracker".to_string()
        }
    }

    fn icon(&self) -> window::Icon {
        if self.state.running_entry.is_some() {
            RUNNING_ICON.clone()
        } else {
            DEFAULT_ICON.clone()
        }
    }

    fn update_icon(&self) -> Command<Message> {
        if let Some(id) = self.window_id {
            window::change_icon(id, self.icon())
        } else {
            Command::none()
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        if let Message::WindowIdReceived(id) = message {
            self.window_id = id;
            if let Some(id) = id {
                return window::change_icon(id, self.icon());
            };
        };

        match &mut self.screen {
            Screen::Loading => match message {
                Message::Loaded(Ok(state)) => {
                    self.screen = Screen::Authed;
                    return Command::future(Self::load_everything(
                        state.api_token,
                    ));
                }
                Message::Loaded(_) => {
                    self.screen = Screen::Unauthed(LoginScreen::new());
                }
                _ => {}
            },
            Screen::Unauthed(screen) => match message {
                Message::LoginProxy(LoginScreenMessage::Completed(Ok(
                    api_token,
                ))) => {
                    self.screen = Screen::Authed;
                    return Command::batch(vec![
                        Command::future(
                            State {
                                api_token: api_token.clone(),
                                ..State::default()
                            }
                            .save(),
                        )
                        .map(|_| Message::Discarded),
                        Command::future(Self::load_everything(api_token)),
                    ]);
                }
                Message::LoginProxy(msg) => {
                    return screen.update(msg).map(Message::LoginProxy)
                }
                _ => {}
            },
            Screen::Authed => {
                if let Message::DataFetched(Ok(state)) = message {
                    self.screen = Screen::Loaded(TemporaryState::default());
                    self.state = state;
                    return self.update_icon();
                }
            }
            Screen::Loaded(temp_state) => match message {
                Message::DataFetched(Ok(state)) => {
                    self.state = state;
                    return self.update_icon();
                }
                Message::TimeEntryProxy(TimeEntryMessage::Edit(i)) => {
                    self.screen = Screen::EditEntry(EditTimeEntry::new(
                        self.state.time_entries[i].clone(),
                        &self.state.api_token,
                    ));
                }
                Message::TimeEntryProxy(TimeEntryMessage::EditRunning) => {
                    if let Some(entry) = &self.state.running_entry {
                        self.screen = Screen::EditEntry(EditTimeEntry::new(
                            entry.clone(),
                            &self.state.api_token,
                        ));
                    }
                }
                Message::TimeEntryProxy(TimeEntryMessage::StopRunning) => {
                    if let Some(entry) = self.state.running_entry.clone() {
                        let token = self.state.api_token.clone();
                        return Command::future(async move {
                            let client = Client::from_api_token(&token);
                            match entry.stop(&client).await {
                                // FIXME: display error
                                Err(e) => {
                                    eprintln!("Stop failed: {e}");
                                    Message::Reload
                                }
                                Ok(_) => Message::Reload,
                            }
                        });
                    }
                }
                Message::SetInitialRunningEntry(description) => {
                    temp_state.new_running_entry_description = description;
                }
                Message::SubmitNewRunningEntry => {
                    let token = self.state.api_token.clone();
                    let description =
                        temp_state.new_running_entry_description.clone();
                    let workspace_id = if self.state.workspaces.is_empty() {
                        eprintln!("No known workspaces");
                        0
                    } else {
                        self.state.workspaces[0].id
                    };
                    return Command::future(async move {
                        let client = Client::from_api_token(&token);
                        let entry =
                            CreateTimeEntry::new(description, workspace_id);
                        match entry.create(&client).await {
                            // FIXME: display error
                            Err(e) => {
                                eprintln!("Submit failed: {e}");
                                Message::Reload
                            }
                            Ok(_) => Message::Reload,
                        }
                    });
                }
                Message::Reload => {
                    *temp_state = TemporaryState::default();
                    return Command::future(Self::load_everything(
                        self.state.api_token.clone(),
                    ));
                }
                _ => {}
            },
            Screen::EditEntry(screen) => match message {
                Message::EditTimeEntryProxy(
                    EditTimeEntryMessage::Completed,
                ) => {
                    self.screen = Screen::Loading;
                    return Command::perform(State::load(), Message::Loaded);
                }
                Message::EditTimeEntryProxy(msg) => {
                    return screen.update(msg).map(Message::EditTimeEntryProxy)
                }
                _ => {}
            },
        };
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        match &self.screen {
            Screen::Loading => loading_message(),
            Screen::Authed => loading_message(),
            Screen::Unauthed(screen) => screen.view().map(Message::LoginProxy),
            Screen::Loaded(temp_state) => {
                let running_entry = match &self.state.running_entry {
                    None => row![text_input(
                        "Create new entry...",
                        &temp_state.new_running_entry_description
                    )
                    .id("running-entry-input")
                    .on_input(Message::SetInitialRunningEntry)
                    .on_submit(Message::SubmitNewRunningEntry)],
                    Some(entry) => {
                        row![entry.view_running().map(Message::TimeEntryProxy)]
                    }
                };
                let content = column(
                    self.state
                        .time_entries
                        .iter()
                        .chunk_by(|e| e.start.date())
                        .into_iter()
                        .map(|(start, tasks)| {
                            column(
                                std::iter::once(
                                    container(
                                        text(date_as_human_readable(start))
                                            .style(text::success),
                                    )
                                    .padding(Padding {
                                        left: 10f32,
                                        ..Padding::default()
                                    })
                                    .into(),
                                )
                                .chain(
                                    tasks.enumerate().flat_map(|(i, task)| {
                                        vec![
                                            task.view(i)
                                                .map(Message::TimeEntryProxy),
                                            horizontal_rule(0.5).into(),
                                        ]
                                    }),
                                ),
                            )
                            .into()
                        }),
                )
                .spacing(10);

                container(
                    column![running_entry, scrollable(content)].spacing(20),
                )
                .center_x(Fill)
                .into()
            }
            Screen::EditEntry(screen) => {
                screen.view().map(Message::EditTimeEntryProxy)
            }
        }
    }

    async fn load_everything(api_token: String) -> Message {
        let client = Client::from_api_token(&api_token);
        ExtendedMe::load(&client)
            .await
            .map(
                |ExtendedMe {
                     api_token,
                     projects,
                     workspaces,
                     time_entries,
                 }| {
                    let (running_entry, time_entries) =
                        TimeEntry::split_running(time_entries);
                    Message::DataFetched(Ok(State {
                        time_entries,
                        running_entry,
                        api_token,
                        projects,
                        workspaces,
                    }))
                },
            )
            .unwrap_or_else(|e| Message::DataFetched(Err(e.to_string())))
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(1))
            .map(|_| Message::Tick)
        // use keyboard::key;
        // keyboard::on_key_press(|key, modifiers| {
        //     let keyboard::Key::Named(key) = key else {
        //         return None;
        //     };

        //     match (key, modifiers) {
        //         (key::Named::Tab, _) => Some(Message::TabPressed {
        //             shift: modifiers.shift(),
        //         }),
        //         (key::Named::ArrowUp, keyboard::Modifiers::SHIFT) => {
        //             Some(Message::ToggleFullscreen(window::Mode::Fullscreen))
        //         }
        //         (key::Named::ArrowDown, keyboard::Modifiers::SHIFT) => {
        //             Some(Message::ToggleFullscreen(window::Mode::Windowed))
        //         }
        //         _ => None,
        //     }
        // })
    }
}

fn loading_message<'a>() -> Element<'a, Message> {
    center(text("Loading...").width(Fill).align_x(Center).size(50)).into()
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

impl State {
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

    async fn load() -> Result<State, LoadError> {
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

        let json = serde_json::to_string_pretty(&self)
            .map_err(|_| SaveError::Format)?;

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

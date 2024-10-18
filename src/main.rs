use components::menu_button;
use customization::{Customization, CustomizationMessage};
use iced::widget::{
    button, center, column, container, horizontal_rule, row, scrollable, text,
    text_input,
};
use iced::{window, Color};
use iced::{Center, Element, Fill, Padding, Task as Command};
use iced_aw::menu;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};

use serde::{Deserialize, Serialize};

mod client;
mod components;
mod customization;
mod edit_time_entry;
mod login;
mod project;
mod related_info;
mod time_entry;
mod workspace;

use crate::client::Client;
use crate::edit_time_entry::{EditTimeEntry, EditTimeEntryMessage};
use crate::login::{LoginScreen, LoginScreenMessage};
use crate::project::{Project, ProjectId};
use crate::related_info::ExtendedMe;
use crate::time_entry::CreateTimeEntry;
use crate::time_entry::{TimeEntry, TimeEntryMessage};
use crate::workspace::{Workspace, WorkspaceId};

pub fn main() -> iced::Result {
    env_logger::init();
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
    has_more_entries: bool,
    projects: Vec<Project>,
    workspaces: Vec<Workspace>,
    default_workspace: Option<WorkspaceId>,
    default_project: Option<ProjectId>,
    customization: Customization,
}

impl State {
    pub fn update_from_context(self, me: ExtendedMe) -> Self {
        let ws_id = self
            .default_workspace
            .filter(|&ws| me.workspaces.iter().any(|w| w.id == ws))
            .or_else(|| me.workspaces.first().map(|ws| ws.id));
        let project_id = self
            .default_project
            .filter(|&proj| me.projects.iter().any(|p| p.id == proj));
        let (running_entry, time_entries) =
            TimeEntry::split_running(if let Some(ws_id) = ws_id {
                me.time_entries
                    .into_iter()
                    .filter(|e| e.workspace_id == ws_id)
                    .collect()
            } else {
                me.time_entries
            });
        Self {
            running_entry,
            time_entries,
            has_more_entries: true,
            projects: me.projects,
            workspaces: me.workspaces,
            default_workspace: ws_id,
            default_project: project_id,
            ..self
        }
    }
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
    error: String,
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
    Loaded(Result<Box<State>, LoadError>),
    DataFetched(Result<ExtendedMe, String>),
    LoginProxy(LoginScreenMessage),
    TimeEntryProxy(TimeEntryMessage),
    EditTimeEntryProxy(EditTimeEntryMessage),
    CustomizationProxy(CustomizationMessage),
    SetInitialRunningEntry(String),
    SubmitNewRunningEntry,
    LoadMore,
    LoadedMore(Vec<TimeEntry>),
    Tick,
    Reload,
    Discarded,
    Error(String),
    WindowIdReceived(Option<window::Id>),
    SelectWorkspace(WorkspaceId),
    SelectProject(Option<ProjectId>),
    TabPressed(bool),
    EscPressed,
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
            Self::default(),
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
        match message {
            Message::WindowIdReceived(id) => {
                debug!("Setting window id to {id:?}");
                self.window_id = id;
                if let Some(id) = id {
                    return window::change_icon(id, self.icon());
                };
            }
            Message::DataFetched(Ok(state)) => {
                info!("Loaded initial data.");
                match &self.screen {
                    Screen::Loaded(_) => {}
                    _ => {
                        self.screen = Screen::Loaded(TemporaryState::default())
                    }
                };
                self.state = self.state.clone().update_from_context(state);
                return Command::batch(vec![
                    Command::future(self.state.clone().save())
                        .map(|_| Message::Discarded),
                    self.update_icon(),
                ]);
            }
            Message::DataFetched(Err(e)) => {
                error!("Failed to fetch initial data: {e}");
                self.error = e;
                return Command::none();
            }
            Message::Error(e) => {
                error!("Received generic error: {e}");
                self.error = e;
                return Command::none();
            }
            _ => {}
        };

        match &mut self.screen {
            Screen::Loading => match message {
                Message::Loaded(Ok(state)) => {
                    info!("Loaded state file.");
                    self.screen = Screen::Authed;
                    let api_token = state.api_token.clone();
                    self.state = *state;
                    return Command::future(Self::load_everything(api_token));
                }
                Message::Loaded(Err(e)) => {
                    error!("Failed to load state file: {e:?}");
                    self.screen = Screen::Unauthed(LoginScreen::new());
                }
                _ => {}
            },
            Screen::Unauthed(screen) => match message {
                Message::LoginProxy(LoginScreenMessage::Completed(
                    api_token,
                )) => {
                    info!("Authenticated successfully.");
                    self.screen = Screen::Authed;
                    self.state = State {
                        api_token: api_token.clone(),
                        ..State::default()
                    };
                    return Command::perform(self.state.clone().save(), |_| {
                        Message::Discarded
                    })
                    .chain(Command::future(Self::load_everything(api_token)));
                }
                Message::LoginProxy(msg) => {
                    return screen.update(msg).map(Message::LoginProxy)
                }
                Message::TabPressed(is_shift) => {
                    return screen
                        .update(LoginScreenMessage::TabPressed(is_shift))
                        .map(Message::LoginProxy)
                }
                _ => {}
            },
            Screen::Authed => {}
            Screen::Loaded(temp_state) => match message {
                Message::TimeEntryProxy(TimeEntryMessage::Edit(i)) => {
                    if let Some(entry) =
                        self.state.time_entries.iter().find(|e| e.id == i)
                    {
                        self.screen = Screen::EditEntry(EditTimeEntry::new(
                            entry.clone(),
                            &self.state.api_token,
                            &self.state.customization,
                            self.state.projects.clone(),
                        ));
                    }
                }
                Message::TimeEntryProxy(TimeEntryMessage::EditRunning) => {
                    if let Some(entry) = &self.state.running_entry {
                        self.screen = Screen::EditEntry(EditTimeEntry::new(
                            entry.clone(),
                            &self.state.api_token,
                            &self.state.customization,
                            self.state.projects.clone(),
                        ));
                    }
                }
                Message::TimeEntryProxy(TimeEntryMessage::StopRunning) => {
                    if let Some(entry) = self.state.running_entry.clone() {
                        info!("Stopping running entry {}...", entry.id);
                        let token = self.state.api_token.clone();
                        return Command::future(async move {
                            let client = Client::from_api_token(&token);
                            match entry.stop(&client).await {
                                Err(e) => {
                                    error!(
                                        "Failed to stop a running entry: {e}"
                                    );
                                    Message::Error(e.to_string())
                                }
                                Ok(_) => {
                                    info!("Entry stopped.");
                                    Message::Reload
                                }
                            }
                        });
                    } else {
                        warn!("Requested to stop a nonexistent running entry.");
                    }
                }
                Message::TimeEntryProxy(TimeEntryMessage::Duplicate(e)) => {
                    let token = self.state.api_token.clone();
                    return Command::future(async move {
                        let client = Client::from_api_token(&token);
                        let entry = CreateTimeEntry::new(
                            e.description,
                            e.workspace_id,
                            e.project_id,
                        );
                        match entry.create(&client).await {
                            Err(e) => {
                                error!("Failed to duplicate an entry: {e}");
                                Message::Error(e.to_string())
                            }
                            Ok(_) => {
                                info!("Entry duplicated.");
                                Message::Reload
                            }
                        }
                    });
                }
                Message::CustomizationProxy(CustomizationMessage::Save) => {
                    return Command::perform(self.state.clone().save(), |_| {
                        Message::Discarded
                    });
                }
                Message::CustomizationProxy(msg) => {
                    return self
                        .state
                        .customization
                        .update(msg)
                        .map(Message::CustomizationProxy);
                }
                Message::SetInitialRunningEntry(description) => {
                    temp_state.new_running_entry_description = description;
                }
                Message::SubmitNewRunningEntry => {
                    let token = self.state.api_token.clone();
                    let description =
                        temp_state.new_running_entry_description.clone();
                    let Some(workspace_id) = self.state.default_workspace
                    else {
                        return Command::done(Message::Error(
                            "No workspace selected!".to_string(),
                        ));
                    };
                    let project_id = self.state.default_project;
                    return Command::future(async move {
                        let client = Client::from_api_token(&token);
                        let entry = CreateTimeEntry::new(
                            Some(description),
                            workspace_id,
                            project_id,
                        );
                        match entry.create(&client).await {
                            Err(e) => {
                                error!("Failed to create a new entry: {e}");
                                Message::Error(e.to_string())
                            }
                            Ok(_) => {
                                info!("Entry created.");
                                Message::Reload
                            }
                        }
                    });
                }
                Message::LoadMore => {
                    info!("Loading older entries...");
                    let token = self.state.api_token.clone();
                    let first_start =
                        self.state.time_entries.last().map(|e| e.start);
                    return Command::future(async move {
                        let client = Client::from_api_token(&token);
                        match TimeEntry::load(first_start, &client).await {
                            Ok(res) => Message::LoadedMore(res),
                            Err(e) => Message::Error(e.to_string()),
                        }
                    });
                }
                Message::LoadedMore(entries) => {
                    info!("Loaded older entries.");
                    if entries.is_empty() {
                        debug!("No older entries.");
                        self.state.has_more_entries = false;
                    }
                    self.state.time_entries.extend(entries.into_iter().filter(
                        |e| {
                            Some(e.workspace_id) == self.state.default_workspace
                        },
                    ));
                    return Command::perform(self.state.clone().save(), |_| {
                        Message::Discarded
                    });
                }
                Message::Reload => {
                    info!("Syncing with remote...");
                    *temp_state = TemporaryState::default();
                    return Command::future(Self::load_everything(
                        self.state.api_token.clone(),
                    ));
                }
                Message::SelectWorkspace(ws_id) => {
                    info!("Selected workspace: {ws_id}");
                    self.state.default_workspace = Some(ws_id);
                    return Command::future(Self::load_everything(
                        self.state.api_token.clone(),
                    ));
                }
                Message::SelectProject(project_id) => {
                    info!("Selected project: {project_id:?}");
                    self.state.default_project = project_id;
                    return Command::perform(self.state.clone().save(), |_| {
                        Message::Discarded
                    });
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
                Message::EscPressed
                | Message::EditTimeEntryProxy(EditTimeEntryMessage::Abort) => {
                    self.screen = Screen::Loaded(TemporaryState::default())
                }
                Message::EditTimeEntryProxy(msg) => {
                    return screen
                        .update(msg, &self.state.customization)
                        .map(Message::EditTimeEntryProxy)
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
                    None => running_entry_input(
                        &temp_state.new_running_entry_description,
                    ),
                    Some(entry) => {
                        entry.view_running().map(Message::TimeEntryProxy)
                    }
                };
                let content = column(
                    self.state
                        .time_entries
                        .iter()
                        .chunk_by(|e| e.start.date_naive())
                        .into_iter()
                        .map(|(start, tasks)| self.day_group(start, tasks)),
                )
                .push(
                    row![button("Load more")
                        .on_press_maybe(if self.state.has_more_entries {
                            Some(Message::LoadMore)
                        } else {
                            None
                        })
                        .style(button::secondary)]
                    .padding([10, 10]),
                );
                let error_repr = if self.error.is_empty() {
                    None
                } else {
                    Some(text(&self.error).style(text::danger))
                };

                container(
                    column![
                        self.menu(),
                        running_entry,
                        container(scrollable(content)).style(|_| {
                            container::Style {
                                border: iced::Border {
                                    color: iced::color!(0x0000cd),
                                    width: 0.5,
                                    radius: 0.into(),
                                },
                                ..container::Style::default()
                            }
                        })
                    ]
                    .push_maybe(error_repr),
                )
                .center_x(Fill)
                .into()
            }
            Screen::EditEntry(screen) => {
                screen.view().map(Message::EditTimeEntryProxy)
            }
        }
    }

    fn menu(&self) -> Element<Message> {
        let selected_ws = self.state.default_workspace;
        let ws_menu = menu::Menu::new(
            self.state
                .workspaces
                .iter()
                .map(|ws| {
                    menu::Item::new(
                        button(text(ws.name.clone()))
                            .width(iced::Length::Fill)
                            .on_press_maybe(if selected_ws == Some(ws.id) {
                                None
                            } else {
                                Some(Message::SelectWorkspace(ws.id))
                            }),
                    )
                })
                .collect(),
        )
        .max_width(200.0);

        let selected_project = self.state.default_project;
        let project_menu = menu::Menu::new(
            std::iter::once(
                menu::Item::<Message, iced::Theme, iced::Renderer>::new(
                    button("None").width(iced::Length::Fill).on_press_maybe(
                        if selected_project.is_none() {
                            None
                        } else {
                            Some(Message::SelectProject(None))
                        },
                    ),
                ),
            )
            .chain(self.state.projects.iter().map(|p| {
                menu::Item::new(
                    button(text(p.name.clone()))
                        .width(iced::Length::Fill)
                        .on_press_maybe(if selected_project == Some(p.id) {
                            None
                        } else {
                            Some(Message::SelectProject(Some(p.id)))
                        }),
                )
            }))
            .collect(),
        )
        .max_width(200.0);

        menu::MenuBar::new(vec![
            menu::Item::with_menu(
                menu_button("Info", Message::Discarded)
                    .width(iced::Length::Fixed(40f32)),
                menu::Menu::new(vec![
                    menu::Item::new(menu_button("Reload", Message::Reload)),
                    menu::Item::with_menu(
                        menu_button("Workspaces", Message::Discarded),
                        ws_menu,
                    ),
                    menu::Item::with_menu(
                        menu_button("Projects", Message::Discarded),
                        project_menu,
                    ),
                ])
                .max_width(120.0),
            ),
            self.state.customization.view(&Message::CustomizationProxy),
        ])
        .into()
    }

    fn day_group<'a>(
        &self,
        start: chrono::NaiveDate,
        tasks: impl Iterator<Item = &'a TimeEntry>,
    ) -> Element<'a, Message> {
        column(
            std::iter::once(
                container(
                    text(self.state.customization.format_date(&start))
                        .style(text::success),
                )
                .padding(Padding {
                    left: 10f32,
                    ..Padding::default()
                })
                .style(|_| container::Style {
                    background: Some(iced::color!(0xc8c8c8).into()),
                    ..container::Style::default()
                })
                .width(iced::Length::Fill)
                .into(),
            )
            .chain(tasks.flat_map(|task| {
                vec![
                    task.view(&self.state.projects)
                        .map(Message::TimeEntryProxy),
                    horizontal_rule(0.5).into(),
                ]
            })),
        )
        .into()
    }

    async fn load_everything(api_token: String) -> Message {
        let client = Client::from_api_token(&api_token);
        ExtendedMe::load(&client)
            .await
            .map(|m| Message::DataFetched(Ok(m)))
            .unwrap_or_else(|e| Message::DataFetched(Err(e.to_string())))
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        use iced::keyboard::{
            key::Named as NamedKey, on_key_press, Key, Modifiers,
        };
        iced::Subscription::batch(vec![
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::Tick),
            on_key_press(|key, modifiers| {
                let Key::Named(key) = key else {
                    return None;
                };
                match (key, modifiers) {
                    (NamedKey::Tab, _) => {
                        if modifiers.bits() == 0 {
                            Some(Message::TabPressed(false))
                        } else if modifiers == Modifiers::SHIFT {
                            Some(Message::TabPressed(true))
                        } else {
                            None
                        }
                    }
                    (NamedKey::Escape, _) if modifiers.bits() == 0 => {
                        Some(Message::EscPressed)
                    }
                    _ => None,
                }
            }),
        ])
    }
}

fn loading_message<'a>() -> Element<'a, Message> {
    center(text("Loading...").width(Fill).align_x(Center).size(50)).into()
}

fn running_entry_input(description: &str) -> Element<'_, Message> {
    row![
        text_input("Create new entry...", description)
            .id("running-entry-input")
            .style(|_, _| text_input::Style {
                background: iced::color!(0x161616).into(),
                border: iced::Border::default(),
                icon: Color::WHITE,
                placeholder: iced::color!(0xd8d8d8),
                value: Color::WHITE,
                selection: Color::WHITE,
            })
            .on_input(Message::SetInitialRunningEntry)
            .on_submit(Message::SubmitNewRunningEntry),
        button("Create").on_press(Message::SubmitNewRunningEntry)
    ]
    .into()
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

    async fn load() -> Result<Box<Self>, LoadError> {
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

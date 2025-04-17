use clap::{crate_version, Parser};
use iced::keyboard::key::Named as NamedKey;
use iced::widget::{
    button, center, column, container, horizontal_rule, row, scrollable, text,
};
use iced::{keyboard, window, Center, Element, Fill, Padding, Task as Command};
use iced_aw::menu;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{debug, error, info};

mod cli;
mod customization;
mod entities;
mod screens;
mod state;
mod time_entry;
mod updater;
mod utils;
mod widgets;

use crate::cli::CliArgs;
use crate::customization::CustomizationMessage;
use crate::entities::{ExtendedMe, ProjectId, WorkspaceId};
use crate::screens::{
    EditTimeEntry, EditTimeEntryMessage, LoginScreen, LoginScreenMessage,
};
use crate::state::{State, StatePersistenceError};
use crate::time_entry::{TimeEntry, TimeEntryMessage};
use crate::updater::UpdateStep;
use crate::utils::{duration_to_hms, Client, ExactModifiers};
use crate::widgets::{
    menu_select_item, menu_text, menu_text_disabled, top_level_menu_text,
    RunningEntry, RunningEntryMessage,
};

pub fn main() -> iced::Result {
    env_logger::init();
    if CliArgs::parse().run().is_some() {
        return Ok(());
    }
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .window_size((400.0, 600.0))
        .settings(iced::Settings {
            default_text_size: 14.into(),
            fonts: vec![
                iced_fonts::BOOTSTRAP_FONT_BYTES.into(),
                iced_fonts::REQUIRED_FONT_BYTES.into(),
            ],
            ..iced::Settings::default()
        })
        .run_with(App::new)
}

#[derive(Debug, Default)]
struct TemporaryState {
    running_entry_widget: RunningEntry,
    update_step: UpdateStep,
}

#[derive(Debug, Default)]
struct App {
    state: State,
    screen: Screen,
    window_id: Option<window::Id>,
    error: String,
}

// There's one instance of this enum at a time, no need to box
#[allow(clippy::large_enum_variant)]
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
    Loaded(Result<Box<State>, StatePersistenceError>),
    DataFetched(ExtendedMe),
    LoginProxy(LoginScreenMessage),
    TimeEntryProxy(TimeEntryMessage),
    EditTimeEntryProxy(EditTimeEntryMessage),
    RunningEntryProxy(RunningEntryMessage),
    CustomizationProxy(CustomizationMessage),
    LoadMore,
    LoadedMore(Vec<TimeEntry>),
    Tick,
    Reload,
    Discarded,
    Error(String),
    WindowIdReceived(Option<window::Id>),
    SelectWorkspace(WorkspaceId),
    SelectProject(Option<ProjectId>),
    KeyPressed(NamedKey, keyboard::Modifiers),
    SetUpdateStep(UpdateStep),
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
            Message::DataFetched(state) => {
                info!("Loaded initial data.");
                if !matches!(&self.screen, Screen::Loaded(_)) {
                    self.screen = Screen::Loaded(TemporaryState::default())
                };
                self.state = self.state.clone().update_from_context(state);
                let mut steps = vec![self.save_state(), self.update_icon()];
                if !self.state.has_whole_last_week() {
                    steps.push(Command::done(Message::LoadMore));
                }
                return Command::batch(steps);
            }
            Message::Error(e) => {
                error!("Received generic error: {e}");
                self.error = e;
                return Command::none();
            }
            Message::KeyPressed(NamedKey::Tab, m) => {
                return if m.is_empty() {
                    iced::widget::focus_next()
                } else if m.shift() && m.is_exact() {
                    iced::widget::focus_previous()
                } else {
                    Command::none()
                }
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
                    error!("Failed to load state file: {e}");
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
                    return self.save_state().chain(Command::future(
                        Self::load_everything(api_token),
                    ));
                }
                Message::LoginProxy(msg) => {
                    return screen.update(msg).map(Message::LoginProxy)
                }
                _ => {}
            },
            Screen::Authed => {}
            Screen::Loaded(temp_state) => match message {
                Message::TimeEntryProxy(TimeEntryMessage::Edit(e)) => {
                    self.begin_edit(e.clone());
                }
                Message::TimeEntryProxy(TimeEntryMessage::Duplicate(e)) => {
                    let token = self.state.api_token.clone();
                    return Command::future(async move {
                        let client = Client::from_api_token(&token);
                        let fut = e.duplicate(&client);
                        match fut.await {
                            Ok(_) => Message::Reload,
                            Err(e) => Message::Error(e.to_string()),
                        }
                    });
                }

                Message::RunningEntryProxy(inner) => match inner {
                    RunningEntryMessage::StartEditing(entry) => {
                        self.begin_edit(*entry.clone());
                    }
                    RunningEntryMessage::Error(err) => {
                        return Command::done(Message::Error(err));
                    }
                    RunningEntryMessage::Reload => {
                        return Command::done(Message::Reload);
                    }
                    other => {
                        return temp_state
                            .running_entry_widget
                            .update(other, &self.state)
                            .map(Message::RunningEntryProxy);
                    }
                },

                Message::CustomizationProxy(inner) => match inner {
                    CustomizationMessage::Save => {
                        return self.save_state();
                    }
                    other => {
                        return self
                            .state
                            .customization
                            .update(other)
                            .map(Message::CustomizationProxy);
                    }
                },

                Message::LoadMore => {
                    info!("Loading older entries...");
                    let token = self.state.api_token.clone();
                    let first_start = self.state.earliest_entry_time;
                    return Command::future(async move {
                        let client = Client::from_api_token(&token);
                        match TimeEntry::load(first_start, &client).await {
                            Ok(res) => Message::LoadedMore(res),
                            Err(e) => Message::Error(e.to_string()),
                        }
                    });
                }
                Message::LoadedMore(entries) => {
                    if entries.is_empty() {
                        info!("No older entries found.");
                    } else {
                        info!("Loaded older entries.");
                    }
                    self.state.add_entries(entries.into_iter());
                    let mut steps = vec![self.save_state(), self.update_icon()];
                    if !self.state.has_whole_last_week() {
                        steps.push(Command::done(Message::LoadMore));
                    }
                    return Command::batch(steps);
                }
                Message::Reload => {
                    debug!("Syncing with remote...");
                    *temp_state = TemporaryState::default();
                    return self.load_entries();
                }
                Message::SelectWorkspace(ws_id) => {
                    self.state.default_workspace = Some(ws_id);
                    return self.save_state().chain(self.load_entries());
                }
                Message::SelectProject(project_id) => {
                    self.state.default_project = project_id;
                    return self.save_state();
                }
                Message::SetUpdateStep(step) => {
                    temp_state.update_step = step;
                    return temp_state
                        .update_step
                        .transition()
                        .map(Message::SetUpdateStep);
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
                Message::EditTimeEntryProxy(EditTimeEntryMessage::Abort) => {
                    self.screen = Screen::Loaded(TemporaryState::default())
                }
                Message::KeyPressed(key, m) => {
                    return screen
                        .handle_key(key, m)
                        .map(Message::EditTimeEntryProxy)
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

    fn save_state(&self) -> Command<Message> {
        Command::future(self.state.clone().save()).map(|_| Message::Discarded)
    }

    fn load_entries(&self) -> Command<Message> {
        Command::future(Self::load_everything(self.state.api_token.clone()))
    }

    fn begin_edit(&mut self, entry: TimeEntry) {
        self.screen = Screen::EditEntry(EditTimeEntry::new(entry, &self.state));
    }

    fn view(&self) -> Element<Message> {
        match &self.screen {
            Screen::Loading => loading_message(),
            Screen::Authed => loading_message(),
            Screen::Unauthed(screen) => screen.view().map(Message::LoginProxy),
            Screen::Loaded(temp_state) => {
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
                        temp_state
                            .running_entry_widget
                            .view(&self.state)
                            .map(Message::RunningEntryProxy),
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
            Screen::EditEntry(screen) => screen
                .view(&self.state.customization)
                .map(Message::EditTimeEntryProxy),
        }
    }

    fn menu(&self) -> Element<Message> {
        let selected_ws = self.state.default_workspace;
        let ws_menu = menu::Menu::new(
            self.state
                .workspaces
                .iter()
                .map(|ws| {
                    menu_select_item(
                        ws.name.clone(),
                        selected_ws == Some(ws.id),
                        Message::SelectWorkspace(ws.id),
                    )
                })
                .collect(),
        )
        .max_width(200.0);

        let selected_project = self.state.default_project;
        let project_menu = menu::Menu::new(
            std::iter::once(menu_select_item(
                "None",
                selected_project.is_none(),
                Message::SelectProject(None),
            ))
            .chain(self.state.projects.iter().map(|p| {
                menu_select_item(
                    &p.name,
                    selected_project == Some(p.id),
                    Message::SelectProject(Some(p.id)),
                )
            }))
            .collect(),
        )
        .max_width(200.0);

        menu::MenuBar::new(vec![
            menu::Item::with_menu(
                top_level_menu_text("Info", Message::Discarded),
                menu::Menu::new(vec![
                    menu::Item::new(menu_text("Reload", Message::Reload)),
                    menu::Item::with_menu(
                        menu_text("Workspaces", Message::Discarded),
                        ws_menu,
                    ),
                    menu::Item::with_menu(
                        menu_text("Projects", Message::Discarded),
                        project_menu,
                    ),
                    menu::Item::new(
                        menu_text_disabled(format!(
                            "Version: {}",
                            crate_version!()
                        ))
                        .padding(iced::Padding {
                            left: 4.0,
                            right: 4.0,
                            top: 6.0,
                            bottom: 4.0,
                        }),
                    ),
                    menu::Item::new(
                        if let Screen::Loaded(state) = &self.screen {
                            iced::Element::from(state.update_step.view())
                                .map(Message::SetUpdateStep)
                        } else {
                            unreachable!(
                                "menu is only present at the loaded screen"
                            )
                        },
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
        let mut total = 0i64;
        let tasks_rendered: Vec<_> = tasks
            .inspect(|task| total += task.duration)
            .flat_map(|task| {
                vec![
                    task.view(&self.state.projects)
                        .map(Message::TimeEntryProxy),
                    horizontal_rule(0.5).into(),
                ]
            })
            .collect();
        const TOP_OFFSET: f32 = 8.0;
        let summary_container_style = |theme: &iced::Theme| {
            let color = theme.extended_palette().secondary.weak;
            container::Style {
                background: Some(color.color.into()),
                text_color: Some(color.text),
                ..container::Style::default()
            }
        };
        column(
            std::iter::once(
                row!(
                    container(text(
                        self.state.customization.format_date(&start)
                    ))
                    .align_left(iced::Length::Shrink)
                    .padding(Padding {
                        left: 10.0,
                        top: TOP_OFFSET,
                        ..Padding::default()
                    })
                    .style(summary_container_style),
                    container(text(duration_to_hms(
                        &chrono::Duration::seconds(total)
                    )))
                    .align_right(iced::Length::Fill)
                    .padding(Padding {
                        right: 20.0,
                        top: TOP_OFFSET,
                        ..Padding::default()
                    })
                    .style(summary_container_style)
                )
                .into(),
            )
            .chain(tasks_rendered),
        )
        .into()
    }

    async fn load_everything(api_token: String) -> Message {
        let client = Client::from_api_token(&api_token);
        ExtendedMe::load(&client)
            .await
            .map(Message::DataFetched)
            .unwrap_or_else(|e| Message::Error(e.to_string()))
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        use iced::keyboard::{on_key_press, Key};
        iced::Subscription::batch(vec![
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::Tick),
            on_key_press(|key, modifiers| {
                let Key::Named(key) = key else {
                    return None;
                };
                match (key, modifiers) {
                    (NamedKey::Enter | NamedKey::Tab, m) => {
                        Some(Message::KeyPressed(key, m))
                    }
                    (NamedKey::Escape, m) if m.is_empty() => {
                        Some(Message::KeyPressed(key, m))
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

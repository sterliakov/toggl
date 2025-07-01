#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unsafe_derive_deserialize)]
#![deny(clippy::shadow_unrelated)]
#![deny(clippy::str_to_string)]
#![deny(clippy::unused_trait_names)]
#![deny(clippy::print_stderr)]
#![deny(clippy::print_stdout)]
#![deny(clippy::filter_map_bool_then)]
#![deny(clippy::if_then_some_else_none)]
#![deny(clippy::return_and_then)]

use std::collections::HashSet;
use std::sync::LazyLock;

use clap::{crate_version, Parser as _};
use entities::Preferences;
use iced::alignment::Horizontal;
use iced::keyboard::key::Named as NamedKey;
use iced::widget::{
    button, center, column, container, horizontal_rule, row, scrollable, text,
};
use iced::{keyboard, window, Center, Element, Fill, Padding, Task as Command};
use iced_aw::menu;
use iced_fonts::Bootstrap;
use itertools::Itertools as _;
use log::{debug, error, info};
use screens::{LegalInfo, LegalInfoMessage};
use state::{EntryEditAction, EntryEditInfo};
use utils::duration_to_hm;
use widgets::{default_button_text, menu_button, menu_icon};

mod cli;
mod customization;
mod entities;
mod screens;
mod state;
#[cfg(test)]
mod test;
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
use crate::utils::{duration_to_hms, Client, ExactModifiers as _};
use crate::widgets::{
    menu_select_item, menu_text, menu_text_disabled, top_level_menu_text,
    CustomWidget as _, RunningEntry, RunningEntryMessage,
};

pub fn main() -> iced::Result {
    env_logger::init();
    if CliArgs::parse().run().is_some() {
        return Ok(());
    }
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
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
#[expect(clippy::large_enum_variant)]
#[derive(Debug, Default)]
enum Screen {
    #[default]
    Loading,
    Unauthed(LoginScreen),
    Loaded(TemporaryState),
    EditEntry(EditTimeEntry),
    Legal(LegalInfo),
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
    LegalProxy(LegalInfoMessage),
    LoadMore,
    LoadedMore(Vec<TimeEntry>),
    Tick,
    Reload,
    Logout(String),
    LogoutDone,
    NewProfile,
    Discarded,
    Error(String),
    WindowIdReceived(Option<window::Id>),
    SelectWorkspace(WorkspaceId),
    SelectProject(Option<ProjectId>),
    SelectProfile(String),
    KeyPressed(NamedKey, keyboard::Modifiers),
    SetUpdateStep(UpdateStep),
    OpenLegalScreen,
    OptimisticUpdate(EntryEditInfo),
}

static RUNNING_ICON: LazyLock<window::Icon> = LazyLock::new(|| {
    window::icon::from_file_data(include_bytes!("../assets/icon.png"), None)
        .expect("Icon must parse")
});

static DEFAULT_ICON: LazyLock<window::Icon> = LazyLock::new(|| {
    window::icon::from_file_data(
        include_bytes!("../assets/icon-gray.png"),
        None,
    )
    .expect("Icon must parse")
});

impl App {
    pub fn new() -> (Self, Command<Message>) {
        (
            Self::default(),
            Command::batch(vec![
                Command::perform(State::load(), Message::Loaded),
                iced::window::get_latest().map(Message::WindowIdReceived),
            ]),
        )
    }

    pub fn title(&self) -> String {
        if self.state.current_profile().running_entry.is_some() {
            "* Toggl Tracker".to_owned()
        } else {
            "Toggl Tracker".to_owned()
        }
    }

    pub fn icon(&self) -> window::Icon {
        if self.state.current_profile().running_entry.is_some() {
            RUNNING_ICON.clone()
        } else {
            DEFAULT_ICON.clone()
        }
    }

    fn update_icon(&self) -> Command<Message> {
        self.window_id.map_or_else(Command::none, |id| {
            window::change_icon(id, self.icon())
        })
    }

    #[expect(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::WindowIdReceived(id) => {
                debug!("Setting window id to {id:?}");
                self.window_id = id;
                if let Some(id) = id {
                    return window::change_icon(id, self.icon());
                }
            }
            Message::DataFetched(context) => {
                info!("Loaded initial data.");
                if !matches!(&self.screen, Screen::Loaded(_)) {
                    self.screen = Screen::Loaded(TemporaryState::default());
                }
                self.state.update_from_context(context);
                let mut steps = vec![self.save_state(), self.update_icon()];
                if !self.state.current_profile().has_whole_last_week() {
                    steps.push(Command::done(Message::LoadMore));
                }
                return Command::batch(steps);
            }
            Message::Logout(profile) => {
                info!("Logging out...");
                let state = self.state.clone();
                return Command::future(async move {
                    state.remove_profile(&profile).await
                })
                .map(|res| match res {
                    Ok(Some(new_state)) => {
                        Message::Loaded(Ok(new_state.into()))
                    }
                    Ok(None) | Err(_) => Message::LogoutDone,
                });
            }
            Message::LogoutDone | Message::NewProfile => {
                self.screen = Screen::Unauthed(LoginScreen::new());
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
            Message::OpenLegalScreen => {
                self.screen = Screen::Legal(LegalInfo::new());
            }
            Message::OptimisticUpdate(change) => {
                return if self.state.apply_change(&change).is_err() {
                    Command::done(Message::Reload)
                } else {
                    Command::batch(vec![self.update_icon(), self.save_state()])
                }
            }
            Message::Loaded(Ok(state)) => {
                info!("Loaded state file.");
                if !matches!(self.screen, Screen::Loaded(_)) {
                    self.screen = Screen::Loaded(TemporaryState::default());
                }
                let api_token = state.api_token();
                self.state = *state;
                return Command::future(Self::load_everything(api_token));
            }
            Message::Loaded(Err(ref e)) => {
                error!("Failed to load state file: {e}");
                self.screen = Screen::Unauthed(LoginScreen::new());
            }
            _ => {}
        }

        match &mut self.screen {
            Screen::Loading => {}
            Screen::Unauthed(screen) => match message {
                Message::LoginProxy(LoginScreenMessage::Completed {
                    email,
                    api_token,
                }) => {
                    info!("Authenticated successfully.");
                    self.screen = Screen::Loading;
                    self.state.ensure_profile(email.clone(), api_token);
                    self.state.select_profile(email);
                    return self.save_state().chain(self.load_entries());
                }
                Message::LoginProxy(msg) => {
                    return screen
                        .update(msg, &self.state)
                        .map(Message::LoginProxy)
                }
                _ => {}
            },
            Screen::Loaded(temp_state) => match message {
                Message::TimeEntryProxy(TimeEntryMessage::Edit(e)) => {
                    self.begin_edit(e);
                }
                Message::TimeEntryProxy(TimeEntryMessage::Duplicate(e)) => {
                    let token = self.state.api_token();
                    return Command::future(async move {
                        let client = Client::from_api_token(&token);
                        let fut = e.duplicate(&client);
                        match fut.await {
                            Ok(new_entry) => {
                                Message::OptimisticUpdate(EntryEditInfo {
                                    action: EntryEditAction::Create,
                                    entry: new_entry,
                                })
                            }
                            Err(err) => Message::Error(err.to_string()),
                        }
                    });
                }

                Message::RunningEntryProxy(inner) => match inner {
                    RunningEntryMessage::StartEditing(entry) => {
                        self.begin_edit(*entry);
                    }
                    RunningEntryMessage::Error(err) => {
                        return Command::done(Message::Error(err));
                    }
                    RunningEntryMessage::SyncUpdate(change) => {
                        return Command::done(Message::OptimisticUpdate(
                            change,
                        ));
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
                        return Command::batch(vec![
                            self.save_state(),
                            self.save_customization(),
                        ]);
                    }
                    other => {
                        return self
                            .state
                            .customization_mut()
                            .update(other)
                            .map(Message::CustomizationProxy);
                    }
                },

                Message::LoadMore => {
                    info!("Loading older entries...");
                    let token = self.state.api_token();
                    let first_start =
                        self.state.current_profile().earliest_entry_time;
                    return Command::future(async move {
                        let client = Client::from_api_token(&token);
                        match TimeEntry::load(first_start, &client).await {
                            Ok(res) => Message::LoadedMore(res),
                            Err(e) => Message::Error(e.to_string()),
                        }
                    });
                }
                Message::LoadedMore(mut entries) => {
                    let already_fetched: HashSet<_> = self
                        .state
                        .current_profile()
                        .time_entries
                        .iter()
                        .map(|e| e.id)
                        .collect();
                    entries.retain(|e| !already_fetched.contains(&e.id));
                    if entries.is_empty() {
                        info!("No older entries found.");
                    } else {
                        info!("Loaded older entries.");
                    }
                    let profile = self.state.current_profile_mut();
                    profile.add_entries(entries.into_iter());
                    let mut steps = vec![self.save_state(), self.update_icon()];
                    if !self.state.current_profile().has_whole_last_week() {
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
                    self.state.current_profile_mut().default_workspace =
                        Some(ws_id);
                    return Command::batch(vec![
                        self.save_state(),
                        self.save_customization(),
                    ])
                    .chain(self.load_entries());
                }
                Message::SelectProject(project_id) => {
                    self.state.current_profile_mut().default_project =
                        project_id;
                    return self.save_state();
                }
                Message::SelectProfile(name) => {
                    self.state.select_profile(name);
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
                    EditTimeEntryMessage::Completed(change),
                ) => {
                    self.screen = Screen::Loaded(TemporaryState::default());
                    return Command::done(Message::OptimisticUpdate(change));
                }
                Message::EditTimeEntryProxy(EditTimeEntryMessage::Abort) => {
                    self.screen = Screen::Loaded(TemporaryState::default());
                }
                Message::EditTimeEntryProxy(msg) => {
                    return screen
                        .update(msg, &self.state)
                        .map(Message::EditTimeEntryProxy)
                }
                Message::KeyPressed(key, m) => {
                    return screen
                        .handle_key(key, m)
                        .map_or_else(Command::none, |t| {
                            t.map(Message::EditTimeEntryProxy)
                        })
                }
                _ => {}
            },
            Screen::Legal(screen) => match message {
                Message::LegalProxy(LegalInfoMessage::Close) => {
                    return self.load_entries();
                }
                Message::LegalProxy(msg) => {
                    return screen
                        .update(msg, &self.state)
                        .map(Message::LegalProxy)
                }
                Message::KeyPressed(key, m) => {
                    return screen
                        .handle_key(key, m)
                        .map_or_else(Command::none, |t| {
                            t.map(Message::LegalProxy)
                        })
                }
                _ => {}
            },
        }
        Command::none()
    }

    fn save_state(&self) -> Command<Message> {
        Command::future(self.state.clone().save()).map(|_| Message::Discarded)
    }

    fn save_customization(&self) -> Command<Message> {
        let state = self.state.clone();
        Command::future(async move {
            match state.current_profile().save_customization().await {
                Ok(()) => Message::Discarded,
                Err(e) => Message::Error(e.to_string()),
            }
        })
    }

    fn load_entries(&self) -> Command<Message> {
        Command::future(Self::load_everything(self.state.api_token()))
    }

    fn begin_edit(&mut self, entry: TimeEntry) {
        self.screen = Screen::EditEntry(EditTimeEntry::new(entry, &self.state));
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.screen {
            Screen::Loading => loading_message(
                &self.error,
                Message::Logout(self.state.active_profile.clone()),
            ),
            Screen::Unauthed(screen) => {
                screen.view(&self.state).map(Message::LoginProxy)
            }
            Screen::Loaded(temp_state) => {
                let content = column(
                    self.state
                        .current_profile()
                        .time_entries
                        .iter()
                        .chunk_by(|e| e.start.date_naive())
                        .into_iter()
                        .map(|(start, tasks)| self.day_group(start, tasks)),
                )
                .push(
                    row![button("Load more")
                        .on_press_maybe(
                            self.state
                                .current_profile()
                                .has_more_entries
                                .then_some(Message::LoadMore)
                        )
                        .style(button::secondary)]
                    .padding([10, 10]),
                );
                let error_repr = if self.error.is_empty() {
                    None
                } else {
                    Some(
                        row![text(&self.error).style(text::danger)]
                            .padding([4, 8]),
                    )
                };

                container(
                    column![self.menu()]
                        .push(
                            temp_state
                                .running_entry_widget
                                .view(&self.state)
                                .map(Message::RunningEntryProxy),
                        )
                        .push_maybe(error_repr)
                        .push(container(scrollable(content)).style(|_| {
                            container::Style {
                                border: iced::Border {
                                    color: iced::color!(0x0000cd),
                                    width: 0.5,
                                    radius: 0.into(),
                                },
                                ..container::Style::default()
                            }
                        })),
                )
                .center_x(Fill)
                .into()
            }
            Screen::EditEntry(screen) => {
                screen.view(&self.state).map(Message::EditTimeEntryProxy)
            }
            Screen::Legal(screen) => {
                screen.view(&self.state).map(Message::LegalProxy)
            }
        }
    }

    fn menu(&self) -> Element<'_, Message> {
        let profile = self.state.current_profile();
        let selected_ws = profile.default_workspace;
        let ws_menu = menu::Menu::new(
            profile
                .workspaces
                .iter()
                .map(|ws| {
                    menu_select_item(
                        &ws.name.clone(),
                        selected_ws == Some(ws.id),
                        Message::SelectWorkspace(ws.id),
                    )
                })
                .collect(),
        )
        .max_width(200.0);

        let selected_project = profile.default_project;
        let project_menu = menu::Menu::new(
            std::iter::once(menu_select_item(
                &"None",
                selected_project.is_none(),
                Message::SelectProject(None),
            ))
            .chain(profile.projects.iter().map(|p| {
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
                top_level_menu_text(&"Info", Message::Discarded),
                menu::Menu::new(vec![
                    menu::Item::new(menu_text(&"Reload", Message::Reload)),
                    menu::Item::with_menu(
                        menu_text(&"Workspaces", Message::Discarded),
                        ws_menu,
                    ),
                    menu::Item::with_menu(
                        menu_text(&"Projects", Message::Discarded),
                        project_menu,
                    ),
                    menu::Item::new(menu_text(
                        &"Legal info",
                        Message::OpenLegalScreen,
                    )),
                    menu::Item::with_menu(
                        menu_text(
                            &format!("Profile: {}", self.state.active_profile),
                            Message::Discarded,
                        ),
                        self.profile_menu(),
                    ),
                    menu::Item::new(
                        menu_text_disabled(&format!(
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
            self.state
                .customization()
                .view(&Message::CustomizationProxy),
            self.week_total(),
        ])
        .width(iced::Length::Fill)
        .style(|theme: &iced::Theme, status: iced_aw::style::Status| {
            menu::Style {
                path_border: iced::Border {
                    radius: 6.0.into(),
                    ..iced::Border::default()
                },
                ..iced_aw::menu::primary(theme, status)
            }
        })
        .into()
    }

    fn profile_menu(
        &self,
    ) -> menu::Menu<'_, Message, iced::Theme, iced::Renderer> {
        let mut items: Vec<_> = self
            .state
            .profile_names()
            .map(|name| {
                menu::Item::new(
                    row![
                        menu_button(
                            default_button_text(&name),
                            (self.state.active_profile != name)
                                .then(|| Message::SelectProfile(name.clone())),
                        ),
                        menu_icon(Bootstrap::Trash)
                            .style(button::danger)
                            .on_press_with(move || Message::Logout(
                                name.clone()
                            ))
                    ]
                    .spacing(4.0)
                    .align_y(iced::alignment::Vertical::Center),
                )
            })
            .collect();
        items.push(menu::Item::new(
            menu_text(&"Add new profile", Message::NewProfile).padding(
                iced::Padding {
                    left: 4.0,
                    right: 4.0,
                    top: 6.0,
                    bottom: 4.0,
                },
            ),
        ));

        menu::Menu::new(items).max_width(200.0)
    }

    fn week_total(
        &self,
    ) -> menu::Item<'_, Message, iced::Theme, iced::Renderer> {
        let duration =
            duration_to_hm(&self.state.current_profile().week_total());
        menu::Item::new(
            menu_button(
                default_button_text(&format!("Week total: {duration}",))
                    .align_x(Horizontal::Right)
                    .width(iced::Length::Fill),
                None,
            )
            .style(|theme, _| button::text(theme, button::Status::Active)),
        )
    }

    fn day_group<'a>(
        &self,
        start: chrono::NaiveDate,
        tasks: impl Iterator<Item = &'a TimeEntry>,
    ) -> Element<'a, Message> {
        const TOP_OFFSET: f32 = 8.0;

        let mut total = 0i64;
        let tasks_rendered: Vec<_> = tasks
            .inspect(|task| total += task.duration)
            .flat_map(|task| {
                vec![
                    task.view(&self.state.current_profile().projects)
                        .map(Message::TimeEntryProxy),
                    horizontal_rule(0.5).into(),
                ]
            })
            .collect();
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
                        self.state.customization().format_date(start)
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
        match tokio::try_join!(
            Preferences::load(&client),
            ExtendedMe::load(&client),
        ) {
            Ok((prefs, mut me)) => {
                me.preferences = prefs;
                Message::DataFetched(me)
            }
            Err(e) => Message::Error(e.to_string()),
        }
    }

    pub fn subscription(_self: &Self) -> iced::Subscription<Message> {
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

    pub fn theme(&self) -> iced::Theme {
        if self.state.customization().dark_mode {
            iced::Theme::Dracula
        } else {
            iced::Theme::Light
        }
    }
}

fn loading_message(error: &str, on_logout: Message) -> Element<'_, Message> {
    if error.is_empty() {
        center(text("Loading...").width(Fill).align_x(Center).size(48)).into()
    } else {
        center(
            column![
                text(format!("Error: {error}"))
                    .width(Fill)
                    .align_x(Center)
                    .size(24),
                button("Log Out")
                    .on_press(on_logout)
                    .style(button::secondary),
            ]
            .align_x(iced::Center)
            .spacing(16.0),
        )
        .into()
    }
}

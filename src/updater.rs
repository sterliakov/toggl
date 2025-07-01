use std::fmt;

use iced::widget::button;
use iced::Task as Command;
use log::{error, info};
use self_update::cargo_crate_version;
use self_update::errors::Error as UpdateError;
pub use self_update::Status as UpdateStatus;

use crate::widgets::{menu_text, menu_text_disabled};

#[derive(Clone, Debug)]
pub enum InstallationMethod {
    Npm,
    Cargo,
    Unknown,
}

impl fmt::Display for InstallationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Npm => "npm",
            Self::Cargo => "cargo",
            Self::Unknown => "the original installation method",
        }
        .fmt(f)
    }
}

impl InstallationMethod {
    pub const fn can_be_updated(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

pub async fn update(confirm: bool) -> Result<UpdateStatus, UpdateError> {
    let isatty = atty::is(atty::Stream::Stdout);
    async_std::task::spawn_blocking(move || {
        self_update::backends::github::Update::configure()
            .repo_owner("sterliakov")
            .repo_name("toggl")
            .bin_name("toggl-tracker")
            .show_download_progress(isatty)
            .no_confirm(!confirm || !isatty)
            .identifier(archive_ident()) // Prevent inclusion of .sha256 files
            .current_version(cargo_crate_version!())
            .build()?
            .update()
    })
    .await
}

pub async fn has_updates() -> Result<bool, UpdateError> {
    let current = cargo_crate_version!();
    async_std::task::spawn_blocking(move || {
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner("sterliakov")
            .repo_name("toggl")
            .build()?
            .fetch()?;
        Ok(releases.iter().any(|r| {
            self_update::version::bump_is_greater(current, &r.version)
                .unwrap_or(false)
        }))
    })
    .await
}

#[cfg(unix)]
const fn archive_ident() -> &'static str {
    ".tar.gz"
}

#[cfg(windows)]
fn archive_ident() -> &'static str {
    ".zip"
}

pub fn guess_installation_method() -> InstallationMethod {
    let Ok(current_exe) = std::env::current_exe() else {
        return InstallationMethod::Unknown;
    };
    if current_exe
        .components()
        .any(|c| c.as_os_str() == "node_modules")
    {
        InstallationMethod::Npm
    } else if current_exe.components().any(|c| c.as_os_str() == ".cargo") {
        InstallationMethod::Cargo
    } else {
        InstallationMethod::Unknown
    }
}

#[derive(Clone, Debug, Default)]
pub enum UpdateStep {
    #[default]
    NotStarted,
    MaybeUnsupported(InstallationMethod),
    Checking,
    UpToDate,
    UpdateAvailable,
    Running,
    Success,
    Error,
}

impl UpdateStep {
    pub fn transition(&self) -> Command<Self> {
        match self {
            Self::Checking => {
                info!("Checking for updates...");
                let method = guess_installation_method();
                Command::future(async move {
                    match has_updates().await {
                        Err(err) => {
                            error!("Failed to update: {err}.");
                            Self::Error
                        }
                        Ok(true) => {
                            if method.can_be_updated() {
                                info!("Update available.");
                                Self::UpdateAvailable
                            } else {
                                info!("Update available. toggl-tracker seems to be installed via {method}");
                                Self::MaybeUnsupported(method)
                            }
                        }
                        Ok(false) => {
                            info!("toggl-track is up to date.");
                            Self::UpToDate
                        }
                    }
                })
            }
            Self::Running => {
                info!("Installing update...");
                Command::future(async {
                    match update(false).await {
                        Err(err) => {
                            error!("Failed to update: {err}.");
                            Self::Error
                        }
                        Ok(UpdateStatus::UpToDate(version)) => {
                            info!("toggl-track {version} is up to date.");
                            Self::UpToDate
                        }
                        Ok(UpdateStatus::Updated(version)) => {
                            info!("toggl-track updated to {version}.");
                            Self::Success
                        }
                    }
                })
            }
            _ => Command::none(),
        }
    }

    pub fn view(
        &self,
    ) -> button::Button<'_, Self, iced::Theme, iced::Renderer> {
        match self {
            Self::NotStarted => menu_text(&"Check for updates", Self::Checking),
            Self::Checking => menu_text_disabled(&"Checking for updates..."),
            Self::UpToDate => menu_text_disabled(&"Up to date."),
            Self::MaybeUnsupported(method) => menu_text(
                &format!("Use {method} to update. Click again to force update"),
                Self::Running,
            )
            .style(button::danger),
            Self::UpdateAvailable => {
                menu_text(&"Update available, click to update", Self::Running)
            }
            Self::Running => menu_text_disabled(&"Installing the update..."),
            Self::Success => {
                menu_text_disabled(&"Updated, please restart the application.")
            }
            Self::Error => {
                menu_text_disabled(&"Failed to update.").style(button::danger)
            }
        }
    }
}

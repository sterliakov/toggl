use self_update::cargo_crate_version;
use self_update::errors::Error as UpdateError;
pub use self_update::Status as UpdateStatus;

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

#[cfg(unix)]
fn archive_ident() -> &'static str {
    ".tar.gz"
}

#[cfg(windows)]
fn archive_ident() -> &'static str {
    ".zip"
}

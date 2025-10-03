use std::io;
use std::sync::Arc;

use iced::futures::channel::mpsc::Sender;
use iced::futures::SinkExt as _;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, Name,
};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::sync::Mutex;

const SOCK_NAME: &str = "toggl.sock";
const SOCK_PATH: &str = "/tmp/toggl.sock";

const PING_MESSAGE: &[u8] = b"PING\n";
const ACK_MESSAGE: &[u8] = b"ACK\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ListenerMessage {
    AnotherStarted,
    Error,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ListenerStartError {
    AlreadyExists,
    Error,
}

impl From<io::Error> for ListenerStartError {
    fn from(_value: io::Error) -> Self {
        Self::Error
    }
}

pub async fn listener(
    report_to: Sender<ListenerMessage>,
) -> Result<(), ListenerStartError> {
    let name = get_sock_name()?;

    let opts = || ListenerOptions::new().name(name.clone());
    let listener = match opts().create_tokio() {
        // If we found a socket in use, that can mean one of two things:
        // either another instance is running (race condition after a preceding check)
        // or it crashed hard and failed to clean up.
        // Ping again to figure out which one is the real cause.
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            log::info!("Socket is occupied by another instance.");
            match ping_other().await {
                Ok(PingResult::Alive) => {
                    return Err(ListenerStartError::AlreadyExists)
                }
                _ if std::fs::exists(SOCK_PATH).is_ok_and(|v| v) => {
                    std::fs::remove_file(SOCK_PATH)?;
                    opts().create_tokio()
                }
                _ => return Err(e.into()),
            }
        }
        v => v,
    }?;

    log::debug!("Singleton server running at {SOCK_NAME}");

    let sender_arc = Arc::new(Mutex::new(report_to));
    loop {
        let mut conn = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Singleton server unable to accept an incoming connection: {e}");
                if let Err(next_err) =
                    sender_arc.lock().await.send(ListenerMessage::Error).await
                {
                    log::error!("Unable to forward notification to the main window: {next_err}");
                }
                continue;
            }
        };

        let sender_arc_clone = sender_arc.clone();
        tokio::spawn(async move {
            let mut buffer = String::with_capacity(6);
            let mut recver = BufReader::new(&conn);
            let msg = if let Err(e) = recver.read_line(&mut buffer).await {
                log::error!("Singleton server unable to read from incoming connection: {e}");
                ListenerMessage::Error
            } else if buffer
                == String::from_utf8(PING_MESSAGE.to_vec()).unwrap()
            {
                if let Err(e) = conn.write_all(ACK_MESSAGE).await {
                    log::error!("Singleton server unable to ack a ping: {e}");
                    ListenerMessage::Error
                } else {
                    log::info!(
                        "Singleton server prevented launching another instance"
                    );
                    ListenerMessage::AnotherStarted
                }
            } else {
                log::warn!("Singleton server received an unrecognized message: {buffer}");
                ListenerMessage::Unknown
            };
            if let Err(e) = sender_arc_clone.lock().await.send(msg).await {
                log::error!(
                    "Unable to forward notification to the main window: {e}"
                );
            }
        });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PingResult {
    Alive,
    Dead,
    NotFound,
    Hacked,
}

#[tokio::main]
pub async fn ping_other_sync() -> io::Result<PingResult> {
    ping_other().await
}

pub async fn ping_other() -> io::Result<PingResult> {
    let name = get_sock_name()?;

    let conn = match tokio::time::timeout(
        std::time::Duration::from_millis(500),
        Stream::connect(name),
    )
    .await
    {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            return if e.kind() == io::ErrorKind::ConnectionRefused {
                log::debug!("No running instance found");
                Ok(PingResult::NotFound)
            } else {
                log::warn!("Unable to connect to a running instance for unknown reason: {e}");
                Err(e)
            }
        }
        Err(_) => {
            // If this times out, we might be looking at a dead old instance.
            // However, it won't actually happen on Linux: connection is implemented
            // via spawn_blocking there.
            log::warn!("Connection to a running instance timed out");
            return Ok(PingResult::Dead);
        }
    };

    // We have connected successfully.
    let (recver, mut sender) = conn.split();

    sender.write_all(PING_MESSAGE).await?;

    let mut buffer = String::with_capacity(6);
    let mut recver = BufReader::new(recver);
    recver.read_line(&mut buffer).await?;

    let check_result = if buffer
        == String::from_utf8(ACK_MESSAGE.to_vec()).unwrap()
    {
        log::info!("Another instance is already running");
        PingResult::Alive
    } else {
        log::warn!("Unrecognized message from a running instance: {buffer}");
        PingResult::Hacked
    };

    drop((recver, sender));

    Ok(check_result)
}

fn get_sock_name() -> io::Result<Name<'static>> {
    if false {
        //GenericNamespaced::is_supported() {
        SOCK_NAME.to_ns_name::<GenericNamespaced>()
    } else {
        SOCK_PATH.to_fs_name::<GenericFilePath>()
    }
}

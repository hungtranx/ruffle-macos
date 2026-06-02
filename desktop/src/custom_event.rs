//! Custom event type for desktop ruffle

use ruffle_core::events::PlayerNotification;
use ruffle_frontend_utils::content::ContentDescriptor;
use serde::Deserialize;

use crate::gui::DialogDescriptor;
use crate::player::{LaunchOptions, PlayerRunnable};

pub enum OpenType {
    File,
    Directory,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ManagedWindowCommand {
    #[serde(rename = "bounds")]
    Bounds {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    #[serde(rename = "visible")]
    Visible { visible: bool },
}

/// User-defined events.
pub enum RuffleEvent {
    /// Indicates that a task is ready to be polled.
    TaskPoll(PlayerRunnable),

    /// Indicates that an asynchronous SWF metadata load has been completed.
    OnMetadata(ruffle_core::swf::HeaderExt),

    /// The user requested to pick and then open a file.
    BrowseAndOpen(Box<LaunchOptions>, OpenType),

    /// The user requested to open a movie.
    Open(ContentDescriptor, Box<LaunchOptions>),

    /// The user requested to close the current SWF.
    CloseFile,

    /// The user requested to enter full screen.
    EnterFullScreen,

    /// The user requested to exit full screen.
    ExitFullScreen,

    /// The user requested to exit Ruffle.
    ExitRequested,

    /// The user selected an item in the right-click context menu.
    ContextMenuItemClicked(usize),

    /// The movie wants to open a dialog.
    OpenDialog(DialogDescriptor),

    /// Ruffle core has a notification to handle.
    PlayerNotification(PlayerNotification),

    /// The host app changed managed window state.
    ManagedWindowCommand(ManagedWindowCommand),

    /// Export Ruffle Bundle from currently playing content and open save dialog.
    ExportBundle,
}

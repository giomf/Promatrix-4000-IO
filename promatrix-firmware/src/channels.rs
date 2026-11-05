//! Channels used to shuttle [`promatrix_protocol::Command`]s and [`promatrix_protocol::Event`]s
//! between tasks.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use promatrix_protocol::{Command, Event};

/// Commands parsed from the USB host, waiting to be applied to the outputs.
pub static COMMANDS: Channel<CriticalSectionRawMutex, Command, 8> = Channel::new();
/// Events waiting to be sent to the USB host.
pub static EVENTS: Channel<CriticalSectionRawMutex, Event, 16> = Channel::new();

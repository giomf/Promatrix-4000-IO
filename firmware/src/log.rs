//! Plain-text logging over the second USB CDC-ACM interface (see
//! [`crate::usb`]). Lines are formatted into a fixed-size buffer and pushed
//! onto a bounded queue; [`log_task`] drains it and writes each line out
//! while the host has the port open. If the queue is full (nobody reading,
//! or the writer falling behind), the newest line is dropped rather than
//! blocking whichever task tried to log.
//!
//! Viewing logs needs no special tooling: the port carries plain ASCII, so
//! e.g. `cat /dev/ttyACM1` (port numbering may vary) works fine.

use core::fmt::Write as _;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_usb::class::cdc_acm::Sender;
use heapless::String;

const LINE_LEN: usize = 120;
const QUEUE_LEN: usize = 8;

static QUEUE: Channel<CriticalSectionRawMutex, String<LINE_LEN>, QUEUE_LEN> = Channel::new();

/// Formats and enqueues a log line; drops it if the queue is full. Use the
/// [`crate::log_info`] / [`crate::log_warn`] macros rather than calling this
/// directly.
pub fn log(level: &str, args: core::fmt::Arguments) {
    let mut line: String<LINE_LEN> = String::new();
    if write!(line, "{level} {args}\n").is_ok() {
        let _ = QUEUE.try_send(line);
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::log::log("INFO", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::log::log("WARN", format_args!($($arg)*))
    };
}

#[embassy_executor::task]
pub async fn log_task(mut sender: Sender<'static, crate::usb::UsbDriver>) {
    let packet_size = sender.max_packet_size() as usize;

    loop {
        sender.wait_connection().await;

        loop {
            let line = QUEUE.receive().await;
            let bytes = line.as_bytes();

            let mut write_err = false;
            let mut was_max_size = false;
            for chunk in bytes.chunks(packet_size) {
                was_max_size = chunk.len() == packet_size;
                if sender.write_packet(chunk).await.is_err() {
                    write_err = true;
                    break;
                }
            }
            // A transfer that's an exact multiple of the max packet size
            // must be terminated with a zero-length packet.
            if !write_err && was_max_size {
                write_err = sender.write_packet(&[]).await.is_err();
            }

            if write_err {
                // Host disconnected; go back to waiting for a connection.
                break;
            }
        }
    }
}

//! Client library for talking to a Promatrix-4000-IO board over the USB
//! CDC-ACM serial port its firmware exposes. See the `promatrix-protocol` crate for
//! the wire format this speaks.

use std::io::{self, BufRead, BufReader, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub use promatrix_protocol::{Command, Event, FIRST_INPUT, NUM_INPUTS, NUM_OUTPUTS};

/// An open connection to the device.
///
/// Commands are sent synchronously through [`Client::set_output`] /
/// [`Client::get_status`]; events (unsolicited input changes, or a status
/// snapshot in response to a `get_status` call) are read off a background
/// thread and delivered through [`Client::recv_event`].
pub struct Client {
    port: Box<dyn serialport::SerialPort>,
    events: mpsc::Receiver<Event>,
}

impl Client {
    /// Open the serial port at `path` (e.g. `/dev/ttyACM0`) and start
    /// listening for events on a background thread.
    pub fn open(path: &str) -> io::Result<Self> {
        let port = serialport::new(path, 115_200)
            // serialport has no "no timeout"; reads that time out are just
            // retried below, a disconnect shows up as a different error.
            .timeout(Duration::from_secs(60 * 60))
            .open()
            .map_err(|e| io::Error::other(format!("opening {path}: {e}")))?;
        let reader = port
            .try_clone()
            .map_err(|e| io::Error::other(format!("cloning {path}: {e}")))?;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line) {
                    // Quiet line, not a disconnect; any partial line read so
                    // far stays in `line` and is completed by the next read.
                    Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                if let Ok(event) = promatrix_protocol::parse_event(&line)
                    && tx.send(event).is_err()
                {
                    break;
                }
                // Unrecognized lines (stray log output, an `ERR ...`
                // response the firmware doesn't currently emit, etc.) are
                // silently dropped rather than surfaced as an error.
                line.clear();
            }
        });

        Ok(Self { port, events: rx })
    }

    /// Set output `output` (1..=8) to `value`.
    pub fn set_output(&mut self, output: u8, value: bool) -> io::Result<()> {
        if output == 0 || output as usize > NUM_OUTPUTS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output out of range (1..=8)",
            ));
        }
        self.send(Command::SetOutput { output, value })
    }

    /// Request a full [`Event::Status`] snapshot. The reply arrives
    /// asynchronously, like any other event, through [`Client::recv_event`].
    pub fn get_status(&mut self) -> io::Result<()> {
        self.send(Command::GetStatus)
    }

    /// Send [`Command::GetStatus`] and block until the reply arrives,
    /// discarding any unsolicited events (e.g. an `InputChanged` that races
    /// ahead of it) received in the meantime.
    pub fn status(&mut self) -> io::Result<(u8, u8)> {
        self.get_status()?;
        loop {
            if let Event::Status { outputs, inputs } = self.recv_event()? {
                return Ok((outputs, inputs));
            }
        }
    }

    fn send(&mut self, command: Command) -> io::Result<()> {
        write!(self.port, "{command}")
    }

    /// Block until the next event arrives from the device.
    pub fn recv_event(&self) -> io::Result<Event> {
        self.events
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "device disconnected"))
    }

    /// Return the next already-queued event, if any, without blocking.
    pub fn try_recv_event(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

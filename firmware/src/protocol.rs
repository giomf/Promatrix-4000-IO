//! Simple line-based, human-readable protocol spoken over the USB CDC-ACM
//! serial port.
//!
//! ## Host -> device
//!
//! ```text
//! SET <channel> <0|1>\n   -- set output <channel> (1..=8) to 0 or 1
//! GET\n                   -- request a full status snapshot
//! ```
//!
//! ## Device -> host
//!
//! ```text
//! EVT IN <channel> <0|1>\n            -- input <channel> (1..=7) changed
//! STATUS OUT <8 bits> IN <7 bits>\n   -- full snapshot, MSB = highest channel
//! ERR <reason>\n                      -- a received command could not be parsed
//! ```
//!
//! The protocol is intentionally minimal; feel free to replace it with
//! something more efficient (e.g. a small binary framing) once the real
//! application requirements are known.

use heapless::String;

use crate::io::{NUM_INPUTS, NUM_OUTPUTS};

/// A parsed command received from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Set output `channel` (0-indexed, 0 == `Out1`) to `value`.
    SetOutput { channel: u8, value: bool },
    /// Request a full [`Event::Status`] snapshot.
    GetStatus,
}

/// A message sent to the host, either unsolicited or in response to a
/// [`Command::GetStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Input `channel` (0-indexed, 0 == `In1`) changed to `value`.
    InputChanged { channel: u8, value: bool },
    /// Full snapshot of all outputs/inputs.
    Status { outputs: u8, inputs: u8 },
}

/// Parse a single line (without the trailing newline) received from the
/// host into a [`Command`].
///
/// Returns `Err` with a short, human-readable reason if the line could not
/// be parsed.
pub fn parse(line: &str) -> Result<Command, &'static str> {
    let line = line.trim();
    let mut parts = line.split_ascii_whitespace();

    match parts.next() {
        Some("SET") => {
            let channel: u8 = parts.next().ok_or("missing channel")?.parse().map_err(|_| "bad channel")?;
            let value = match parts.next().ok_or("missing value")? {
                "1" => true,
                "0" => false,
                _ => return Err("bad value"),
            };
            if channel == 0 || channel as usize > NUM_OUTPUTS {
                return Err("channel out of range");
            }
            Ok(Command::SetOutput { channel: channel - 1, value })
        }
        Some("GET") => Ok(Command::GetStatus),
        Some(_) => Err("unknown command"),
        None => Err("empty line"),
    }
}

/// Format an [`Event`] as a `\n`-terminated line ready to be sent to the
/// host.
pub fn format(event: &Event) -> String<64> {
    let mut out = String::new();
    match *event {
        Event::InputChanged { channel, value } => {
            let _ = write_line(&mut out, format_args!("EVT IN {} {}\n", channel + 1, value as u8));
        }
        Event::Status { outputs, inputs } => {
            let _ = write_line(
                &mut out,
                format_args!(
                    "STATUS OUT {} IN {}\n",
                    Bits(outputs, NUM_OUTPUTS),
                    Bits(inputs, NUM_INPUTS)
                ),
            );
        }
    }
    out
}

fn write_line(out: &mut String<64>, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
    use core::fmt::Write;
    out.write_fmt(args)
}

/// Helper to format the lowest `count` bits of `value` as a string of `0`/`1`
/// characters, most-significant (highest channel) bit first.
struct Bits(u8, usize);

impl core::fmt::Display for Bits {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Bits(value, count) = *self;
        for i in (0..count).rev() {
            let bit = (value >> i) & 1;
            write!(f, "{}", bit)?;
        }
        Ok(())
    }
}

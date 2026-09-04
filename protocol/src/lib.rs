//! Line-based, human-readable wire protocol spoken over the USB CDC-ACM
//! serial port the firmware (`../firmware`) exposes. Shared between the
//! firmware and the host control tool (`../control`) so the two can never
//! drift apart.
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
//! ```
//!
//! Channels are 1-indexed on the wire (`Out1`..`Out8`, `In1`..`In7`) but
//! 0-indexed in [`Command`]/[`Event`] (to match array indexing on the
//! firmware side); the `Display` impls and `parse_*` functions below do the
//! conversion at the wire boundary.
//!
//! The protocol is intentionally minimal; feel free to replace it with
//! something more efficient (e.g. a small binary framing) once the real
//! application requirements are known.

#![no_std]

#[cfg(test)]
extern crate std;

use core::fmt;

/// Number of digital outputs (`Out1..=Out8`).
pub const NUM_OUTPUTS: usize = 8;
/// Number of digital inputs actually wired on the board (`In1..=In7`).
pub const NUM_INPUTS: usize = 7;

/// A command sent from the host to the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Set output `channel` (0-indexed, 0 == `Out1`) to `value`.
    SetOutput { channel: u8, value: bool },
    /// Request a full [`Event::Status`] snapshot.
    GetStatus,
}

/// A message sent from the device to the host, either unsolicited or in
/// response to a [`Command::GetStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Input `channel` (0-indexed, 0 == `In1`) changed to `value`.
    InputChanged { channel: u8, value: bool },
    /// Full snapshot of all outputs/inputs.
    Status { outputs: u8, inputs: u8 },
}

/// Parse a single line (with or without a trailing newline) received from
/// the host into a [`Command`].
///
/// Returns `Err` with a short, human-readable reason if the line could not
/// be parsed.
pub fn parse_command(line: &str) -> Result<Command, &'static str> {
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

impl fmt::Display for Command {
    /// Formats as a `\n`-terminated line ready to send to the device.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Command::SetOutput { channel, value } => writeln!(f, "SET {} {}", channel + 1, value as u8),
            Command::GetStatus => writeln!(f, "GET"),
        }
    }
}

/// Parse a single line (with or without a trailing newline) received from
/// the device into an [`Event`].
///
/// Returns `Err` with the trimmed, unrecognized line if it doesn't match a
/// known device -> host message.
pub fn parse_event(line: &str) -> Result<Event, &str> {
    let trimmed = line.trim();
    parse_event_inner(trimmed).ok_or(trimmed)
}

fn parse_event_inner(line: &str) -> Option<Event> {
    let mut parts = line.split_ascii_whitespace();
    match parts.next()? {
        "EVT" => {
            if parts.next()? != "IN" {
                return None;
            }
            let channel: u8 = parts.next()?.parse().ok()?;
            let value = match parts.next()? {
                "1" => true,
                "0" => false,
                _ => return None,
            };
            if channel == 0 || channel as usize > NUM_INPUTS {
                return None;
            }
            Some(Event::InputChanged { channel: channel - 1, value })
        }
        "STATUS" => {
            if parts.next()? != "OUT" {
                return None;
            }
            let outputs = parse_bits(parts.next()?, NUM_OUTPUTS)?;
            if parts.next()? != "IN" {
                return None;
            }
            let inputs = parse_bits(parts.next()?, NUM_INPUTS)?;
            Some(Event::Status { outputs, inputs })
        }
        _ => None,
    }
}

fn parse_bits(s: &str, count: usize) -> Option<u8> {
    if s.len() != count {
        return None;
    }
    let mut value = 0u8;
    for b in s.bytes() {
        let bit = match b {
            b'0' => 0,
            b'1' => 1,
            _ => return None,
        };
        value = (value << 1) | bit;
    }
    Some(value)
}

impl fmt::Display for Event {
    /// Formats as a `\n`-terminated line ready to send to the host.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Event::InputChanged { channel, value } => writeln!(f, "EVT IN {} {}", channel + 1, value as u8),
            Event::Status { outputs, inputs } => {
                writeln!(f, "STATUS OUT {} IN {}", Bits(outputs, NUM_OUTPUTS), Bits(inputs, NUM_INPUTS))
            }
        }
    }
}

/// Helper to format the lowest `count` bits of `value` as a string of `0`/`1`
/// characters, most-significant (highest channel) bit first.
struct Bits(u8, usize);

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Bits(value, count) = *self;
        for i in (0..count).rev() {
            let bit = (value >> i) & 1;
            write!(f, "{}", bit)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_commands() {
        assert_eq!(parse_command("SET 3 1"), Ok(Command::SetOutput { channel: 2, value: true }));
        assert_eq!(parse_command("SET 8 0"), Ok(Command::SetOutput { channel: 7, value: false }));
        assert_eq!(parse_command("GET"), Ok(Command::GetStatus));
        assert_eq!(parse_command("SET 9 1"), Err("channel out of range"));
        assert_eq!(parse_command("SET 0 1"), Err("channel out of range"));
        assert_eq!(parse_command("nonsense"), Err("unknown command"));

        assert_eq!(
            std::format!("{}", Command::SetOutput { channel: 2, value: true }),
            "SET 3 1\n"
        );
        assert_eq!(std::format!("{}", Command::GetStatus), "GET\n");
    }

    #[test]
    fn round_trips_events() {
        assert_eq!(parse_event("EVT IN 4 1"), Ok(Event::InputChanged { channel: 3, value: true }));
        assert_eq!(
            parse_event("STATUS OUT 10000000 IN 0000001\n"),
            Ok(Event::Status { outputs: 0b1000_0000, inputs: 0b0000_0001 })
        );
        assert_eq!(parse_event(""), Err(""));
        assert_eq!(parse_event("EVT IN x 1"), Err("EVT IN x 1"));

        assert_eq!(
            std::format!("{}", Event::InputChanged { channel: 3, value: true }),
            "EVT IN 4 1\n"
        );
        assert_eq!(
            std::format!("{}", Event::Status { outputs: 0b1000_0000, inputs: 0b0000_0001 }),
            "STATUS OUT 10000000 IN 0000001\n"
        );
    }
}

//! `promatrix-io` — command-line control tool for a Promatrix-4000-IO board.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use promatrix_io::{Client, Event, FIRST_INPUT, NUM_INPUTS, NUM_OUTPUTS};

#[derive(Parser)]
#[command(about = "Talk to a Promatrix-4000-IO board over its USB serial port")]
struct Cli {
    /// Serial port the board's data interface is on, e.g. /dev/ttyACM0.
    #[arg(short, long, env = "PROMATRIX_PORT")]
    port: String,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Set output <output> (1..=8) to 0 or 1.
    Set {
        /// 1..=8
        output: u8,
        /// 0/1, on/off, or true/false
        #[arg(value_parser = parse_bool, action = clap::ArgAction::Set)]
        value: bool,
    },
    /// Request and print a single status snapshot, then exit.
    Get,
    /// Print a status snapshot, then stream input-change events until
    /// interrupted (Ctrl-C).
    Watch,
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "on" | "true" => Ok(true),
        "0" | "off" | "false" => Ok(false),
        _ => Err(format!("expected 0/1/on/off, got {s:?}")),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut client = Client::open(&cli.port).with_context(|| format!("opening {}", cli.port))?;

    match cli.command {
        Cmd::Set { output, value } => client.set_output(output, value)?,
        Cmd::Get => print_status(client.status()?),
        Cmd::Watch => {
            print_status(client.status()?);
            loop {
                print_event(client.recv_event()?);
            }
        }
    }
    Ok(())
}

fn print_event(event: Event) {
    match event {
        Event::InputChanged { input, value } => {
            println!("In{input}  {}", if value { "on" } else { "off" });
        }
        Event::Status { outputs, inputs } => print_status((outputs, inputs)),
    }
}

fn print_status((outputs, inputs): (u8, u8)) {
    println!("Outputs:");
    print_channels("Out", outputs, 1, NUM_OUTPUTS);
    println!("Inputs:");
    print_channels("In", inputs, FIRST_INPUT, NUM_INPUTS);
}

/// Print one `  <prefix><channel>  on|off` line per channel, `<channel>`
/// counting up from `first` at the least-significant bit of `value`.
fn print_channels(prefix: &str, value: u8, first: u8, count: usize) {
    for i in 0..count {
        let label = format!("{prefix}{}", i + first as usize);
        let state = if (value >> i) & 1 == 1 { "on" } else { "off" };
        println!("  {label:<5}{state}");
    }
}

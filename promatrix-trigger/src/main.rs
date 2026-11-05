//! `promatrix-trigger` — reacts to input changes on a Promatrix-4000-IO board.

use std::path::PathBuf;

use am03127_client::PanelClient;
use am03127_client::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use promatrix_io::{Client, Event};

#[derive(Parser)]
#[command(about = "Trigger actions on Promatrix-4000-IO input changes")]
struct Cli {
    /// Serial port the board's data interface is on, e.g. /dev/ttyACM0.
    #[arg(short, long, env = "PROMATRIX_PORT")]
    port: String,

    /// AM03127 panel config (same TOML the `am03127-cli` uses).
    #[arg(long, env = "PROMATRIX_PANEL_CONFIG")]
    panel_config: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Same selection as `am03127-cli` without `--panels`: all non-hidden ones.
    let config = Config::from_file(&cli.panel_config)?;
    let panels: Vec<PanelClient> = config
        .select_panels(&[])?
        .iter()
        .map(|p| PanelClient::new(&p.address))
        .collect();
    println!("{} panels configured", panels.len());

    let mut client = Client::open(&cli.port).with_context(|| format!("opening {}", cli.port))?;

    // Initial snapshot, so we know where every input starts.
    let (_outputs, inputs) = client.status()?;
    println!("initial inputs: {inputs:07b}");

    loop {
        if let Event::InputChanged { input, value } = client.recv_event()? {
            on_input(&mut client, &panels, input, value)?;
        }
    }
}

// Inputs are the Promatrix 4000's outputs. "Toggle" lines hold their level,
// the others are ~1 s pulses, so only their rising edge matters.
/// Alarmanlage (toggle): on = alarm active.
const IN_ALARM: u8 = 2;
/// TME ausgelöst (1 s pulse).
const IN_TME: u8 = 3;
/// Alarm bestätigt via Alarmtaster (toggle).
const IN_ALARM_ACK: u8 = 4;

/// Called once per input edge. `input` is the board label number (2..=8).
fn on_input(_client: &mut Client, _panels: &[PanelClient], input: u8, value: bool) -> Result<()> {
    match (input, value) {
        (IN_ALARM, true) => println!("alarm on"),
        (IN_ALARM, false) => println!("alarm off"),
        (IN_TME, true) => println!("TME triggered"),
        (IN_ALARM_ACK, true) => println!("alarm acknowledged"),
        (IN_ALARM_ACK, false) => println!("alarm acknowledgement cleared"),
        _ => println!("In{input} {}", if value { "on" } else { "off" }),
    }
    Ok(())
}

//! `promatrix-trigger` — reacts to input changes on a Promatrix-4000-IO board.

use std::path::PathBuf;
use std::time::Duration;

use am03127_client::PanelClient;
use am03127_client::config::Config;
use am03127_commands::schedule::{Schedule, ScheduleDateTime};
use anyhow::{Context, Result, anyhow};
use clap::Parser;
use promatrix_io::{Client, Event, FIRST_INPUT};
use tokio::task::JoinSet;

/// Alarm system (toggle, a Promatrix 4000 output): on = armed, building empty.
const IN_ALARM: u8 = 2;

/// Panel schedule slot driven by the alarm system, and the page it shows.
const SCHEDULE_ID: char = 'A';
const SCHEDULE_PAGE: char = 'C';

/// Per-panel request timeout, so an offline panel can't stall the event loop.
const PANEL_TIMEOUT: Duration = Duration::from_secs(10);

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

struct Panel {
    name: String,
    client: PanelClient,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Same selection as `am03127-cli` without `--panels`: all non-hidden ones.
    let config = Config::from_file(&cli.panel_config)?;
    let panels: Vec<Panel> = config
        .select_panels(&[])?
        .iter()
        .map(|p| Panel {
            name: p.name.clone(),
            client: PanelClient::new(&p.address),
        })
        .collect();
    println!("{} panels configured", panels.len());

    // The panel client is async; the event loop stays blocking and just
    // waits for each panel update to finish.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let mut client = Client::open(&cli.port).with_context(|| format!("opening {}", cli.port))?;

    // Bring the panels in line with the current state, e.g. after a restart.
    let (_outputs, inputs) = client.status()?;
    let armed = (inputs >> (IN_ALARM - FIRST_INPUT)) & 1 == 1;
    println!("initial alarm system {}", on_off(armed));
    rt.block_on(set_alarm(&panels, armed));

    loop {
        if let Event::InputChanged { input, value } = client.recv_event()? {
            println!("In{input} {}", on_off(value));
            if input == IN_ALARM {
                rt.block_on(set_alarm(&panels, value));
            }
        }
    }
}

/// Armed: delete the schedule, so the panels fall back to their (empty)
/// default page. Disarmed: show the schedule's page, from now on until 2099.
async fn set_alarm(panels: &[Panel], armed: bool) {
    let always = Schedule::new(
        SCHEDULE_ID,
        ScheduleDateTime {
            year: 0,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
        },
        ScheduleDateTime {
            year: 99,
            month: 12,
            day: 31,
            hour: 23,
            minute: 59,
        },
        SCHEDULE_PAGE.to_string(),
    );

    let mut set = JoinSet::new();
    for panel in panels {
        let (name, client, schedule) = (panel.name.clone(), panel.client.clone(), always.clone());
        set.spawn(async move {
            let request = async {
                if armed {
                    client.delete_schedule(SCHEDULE_ID).await
                } else {
                    client.set_schedule(&schedule).await
                }
            };
            let result = tokio::time::timeout(PANEL_TIMEOUT, request)
                .await
                .unwrap_or_else(|_| Err(anyhow!("timed out")));
            (name, result)
        });
    }

    // One unreachable panel must not stop the others or the trigger itself.
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((name, Err(e))) => eprintln!("{name}: {e:#}"),
            Err(e) => eprintln!("panel task failed: {e}"),
            Ok(_) => {}
        }
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

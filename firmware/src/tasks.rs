//! Async tasks: USB read/write pumps, output command handling, and input
//! change detection.

use embassy_rp::gpio::Input;
use embassy_time::Timer;
use embassy_usb::driver::EndpointError;

use crate::channels::{COMMANDS, EVENTS};
use crate::io::Outputs;
use crate::protocol::{self, Command, Event};
use crate::state;
use crate::usb::UsbDriver;

/// Reads lines from the USB host, parses them and pushes the resulting
/// [`Command`]s onto [`COMMANDS`].
#[embassy_executor::task]
pub async fn usb_reader_task(mut receiver: embassy_usb::class::cdc_acm::Receiver<'static, UsbDriver>) {
    let mut line: heapless::Vec<u8, 128> = heapless::Vec::new();
    let mut pkt = [0u8; 64];

    loop {
        receiver.wait_connection().await;
        crate::log_info!("USB host connected");
        line.clear();

        loop {
            match receiver.read_packet(&mut pkt).await {
                Ok(n) => {
                    for &b in &pkt[..n] {
                        if b == b'\n' || b == b'\r' {
                            if !line.is_empty() {
                                if let Ok(text) = core::str::from_utf8(&line) {
                                    match protocol::parse(text) {
                                        Ok(cmd) => {
                                            if COMMANDS.try_send(cmd).is_err() {
                                                crate::log_warn!("command queue full, dropping command");
                                            }
                                        }
                                        Err(reason) => crate::log_warn!("bad command received: {reason} ({text:?})"),
                                    }
                                }
                                line.clear();
                            }
                        } else if line.push(b).is_err() {
                            crate::log_warn!("line too long, discarding");
                            line.clear();
                        }
                    }
                }
                Err(EndpointError::Disabled) => {
                    crate::log_info!("USB host disconnected");
                    break;
                }
                Err(EndpointError::BufferOverflow) => crate::log_warn!("USB read buffer overflow"),
            }
        }
    }
}

/// Pulls [`Event`]s off [`EVENTS`] and writes them to the USB host.
#[embassy_executor::task]
pub async fn usb_writer_task(mut sender: embassy_usb::class::cdc_acm::Sender<'static, UsbDriver>) {
    loop {
        let event = EVENTS.receive().await;
        let line = protocol::format(&event);

        if sender.write_packet(line.as_bytes()).await.is_err() {
            // Host not connected (or gone away); drop the event rather than
            // blocking and backing up the channel.
            continue;
        }
        if line.len() % 64 == 0 {
            // A transfer that's an exact multiple of the max packet size
            // must be terminated with a zero-length packet.
            let _ = sender.write_packet(&[]).await;
        }
    }
}

/// Applies [`Command`]s from [`COMMANDS`] to the physical outputs, and
/// answers [`Command::GetStatus`] with a snapshot [`Event::Status`].
#[embassy_executor::task]
pub async fn output_task(mut outputs: Outputs) {
    loop {
        match COMMANDS.receive().await {
            Command::SetOutput { channel, value } => {
                crate::log_info!("OUT{} set to {}", channel + 1, value as u8);
                outputs.set(channel, value);
                state::set_output(channel, value);
            }
            Command::GetStatus => {
                let event = Event::Status { outputs: state::outputs(), inputs: state::inputs() };
                if EVENTS.try_send(event).is_err() {
                    crate::log_warn!("event queue full, dropping status snapshot");
                }
            }
        }
    }
}

/// Watches a single input pin for changes and reports them, debounced, on
/// [`EVENTS`]. One instance of this task is spawned per wired input.
#[embassy_executor::task(pool_size = 7)]
pub async fn input_task(mut pin: Input<'static>, channel: u8) {
    // Electrical low == optocoupler conducting == logical "active" (1).
    let mut active = pin.is_low();
    state::set_input(channel, active);

    loop {
        pin.wait_for_any_edge().await;
        // Simple debounce: let the level settle before trusting it.
        Timer::after_millis(5).await;

        let now_active = pin.is_low();
        if now_active != active {
            active = now_active;
            state::set_input(channel, active);
            crate::log_info!("IN{} changed to {}", channel + 1, active as u8);
            if EVENTS.try_send(Event::InputChanged { channel, value: active }).is_err() {
                crate::log_warn!("event queue full, dropping input change event");
            }
        }
    }
}

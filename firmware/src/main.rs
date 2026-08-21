//! Promatrix-4000-IO firmware.
//!
//! Targets the Waveshare RP2040-Zero board. Controls 8 digital outputs
//! (through a ULN2803A Darlington array) and senses 7 digital inputs
//! (through PC817 optocouplers) in near real-time, exposing both over a USB
//! CDC-ACM ("virtual serial port") connection to a host PC. See
//! [`protocol`] for the wire format and [`io`] for the physical pin mapping.

#![no_std]
#![no_main]

mod channels;
mod io;
mod log;
mod protocol;
mod reset;
mod state;
mod tasks;
mod usb;

use embassy_executor::Spawner;
use panic_halt as _;

use crate::io::{InputPins, Inputs, OutputPins, Outputs};
use crate::usb::Usb;

#[embassy_executor::main(executor = "embassy_rp::executor::Executor", entry = "cortex_m_rt::entry")]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    crate::log_info!("Promatrix-4000-IO firmware starting");

    let outputs = Outputs::new(OutputPins {
        out1: p.PIN_0,
        out2: p.PIN_1,
        out3: p.PIN_2,
        out4: p.PIN_3,
        out5: p.PIN_4,
        out6: p.PIN_5,
        out7: p.PIN_6,
        out8: p.PIN_7,
    });

    let Inputs(input_pins) = Inputs::new(InputPins {
        in1: p.PIN_10,
        in2: p.PIN_9,
        in3: p.PIN_8,
        in4: p.PIN_14,
        in5: p.PIN_13,
        in6: p.PIN_12,
        in7: p.PIN_11,
    });

    let Usb { device, sender, receiver, log_sender } = usb::init(p.USB, p.FLASH);

    spawner.spawn(usb::usb_task(device).unwrap());
    spawner.spawn(log::log_task(log_sender).unwrap());
    spawner.spawn(tasks::usb_reader_task(receiver).unwrap());
    spawner.spawn(tasks::usb_writer_task(sender).unwrap());
    spawner.spawn(tasks::output_task(outputs).unwrap());

    for (channel, pin) in input_pins.into_iter().enumerate() {
        spawner.spawn(tasks::input_task(pin, channel as u8).unwrap());
    }
}

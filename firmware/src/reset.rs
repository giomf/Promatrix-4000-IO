//! Vendor "reset interface", compatible with `picotool`'s `--force`/`-f`
//! flag, so flashing doesn't require holding the BOOT button.
//!
//! `picotool load -f` looks for a USB interface with class `0xFF`, subclass
//! `0x00` and protocol `0x01` (see the Pico SDK's
//! `pico/usb_reset_interface.h`) and sends it one of two vendor control
//! requests: reboot into BOOTSEL mode, or reboot back into the flashed
//! application. This mirrors that protocol so picotool can drive it without
//! the physical button.

use embassy_usb::Handler;
use embassy_usb::control::{OutResponse, Recipient, Request, RequestType};
use embassy_usb::types::InterfaceNumber;

/// USB interface class/subclass/protocol picotool scans for.
pub const CLASS: u8 = 0xff;
pub const SUBCLASS: u8 = 0x00;
pub const PROTOCOL: u8 = 0x01;

const REQUEST_BOOTSEL: u8 = 0x01;
const REQUEST_FLASH: u8 = 0x02;

pub struct ResetHandler {
    interface: u8,
}

impl ResetHandler {
    pub fn new(interface: InterfaceNumber) -> Self {
        Self { interface: interface.into() }
    }
}

impl Handler for ResetHandler {
    fn control_out(&mut self, req: Request, _data: &[u8]) -> Option<OutResponse> {
        if req.request_type != RequestType::Class
            || req.recipient != Recipient::Interface
            || req.index != self.interface as u16
        {
            return None;
        }

        match req.request {
            REQUEST_BOOTSEL => {
                // The low bits of `wValue` are an interface-disable mask (see
                // `embassy_rp::rom_data::reset_to_usb_boot`); picotool sends 0
                // here unless asked to customize the BOOTSEL drive, which we
                // don't need to support.
                embassy_rp::rom_data::reset_to_usb_boot(0, req.value as u32);
                Some(OutResponse::Accepted) // unreachable: the above resets the chip
            }
            REQUEST_FLASH => cortex_m::peripheral::SCB::sys_reset(),
            _ => Some(OutResponse::Rejected),
        }
    }
}

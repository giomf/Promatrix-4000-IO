//! USB CDC-ACM ("virtual serial port") setup.
//!
//! The RP2040 enumerates as a composite USB device exposing two CDC-ACM
//! ("virtual serial port") interfaces: the first carries the line-based
//! protocol described in [`protocol`], the second carries plain-text
//! logs (see [`crate::log`]). Talk to the first with any regular serial
//! terminal/library (e.g. Python's `pyserial`).

use core::fmt::Write as _;

use embassy_rp::Peri;
use embassy_rp::bind_interrupts;
use embassy_rp::flash::{Blocking, Flash};
use embassy_rp::peripherals::{FLASH, USB};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_usb::UsbDevice;
use embassy_usb::class::cdc_acm::{CdcAcmClass, Receiver, Sender, State};
use static_cell::StaticCell;

use crate::reset::{self, ResetHandler};

/// Size of the onboard flash chip (see `memory.x`); required by
/// [`embassy_rp::flash::Flash`]'s type but irrelevant to reading the unique ID.
const FLASH_SIZE: usize = 2 * 1024 * 1024;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

/// Concrete driver type for the RP2040's USB peripheral.
pub type UsbDriver = Driver<'static, USB>;

/// Maximum size (in bytes) of a single USB bulk packet on the CDC-ACM data
/// endpoints. 64 is the maximum allowed for full-speed USB and is what most
/// RP2040 examples use.
const MAX_PACKET_SIZE: u16 = 64;

/// Everything needed to run the USB stack: the [`UsbDevice`] future (poll it
/// in its own task via [`UsbDevice::run`]) plus a split sender/receiver pair
/// for the CDC-ACM serial port, plus the sender half of a second CDC-ACM
/// interface dedicated to plain-text logs (see [`crate::log`]).
pub struct Usb {
    pub device: UsbDevice<'static, UsbDriver>,
    pub sender: Sender<'static, UsbDriver>,
    pub receiver: Receiver<'static, UsbDriver>,
    pub log_sender: Sender<'static, UsbDriver>,
}

/// Build the USB device and CDC-ACM class.
///
/// NOTE: the VID/PID pair below (`16c0:27dd`) is the well-known
/// [pid.codes](https://pid.codes) "Test PID"; it is fine for development but
/// should be replaced with a properly allocated VID/PID pair (or one from
/// pid.codes) before this becomes a real product.
pub fn init(usb: Peri<'static, USB>, flash: Peri<'static, FLASH>) -> Usb {
    let driver = Driver::new(usb, Irqs);

    // Use the flash chip's unique ID as the USB serial number: the RP2040
    // bootrom reports the very same ID when running the BOOTSEL bootloader,
    // which is what lets `picotool load -f` (see `crate::reset`) find the
    // device again by serial number after the reboot it triggers.
    static SERIAL: StaticCell<heapless::String<16>> = StaticCell::new();
    let serial = SERIAL.init_with(|| {
        let mut id = [0u8; 8];
        Flash::<_, Blocking, FLASH_SIZE>::new_blocking(flash)
            .blocking_unique_id(&mut id)
            .unwrap();
        let mut serial = heapless::String::new();
        for byte in id {
            let _ = write!(serial, "{byte:02X}");
        }
        serial
    });

    let mut config = embassy_usb::Config::new(0x16c0, 0x27dd);
    config.manufacturer = Some("Promatrix");
    config.product = Some("Promatrix-4000-IO");
    config.serial_number = Some(serial.as_str());
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static STATE: StaticCell<State> = StaticCell::new();
    static LOG_STATE: StaticCell<State> = StaticCell::new();

    let mut builder = embassy_usb::Builder::new(
        driver,
        config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [], // no MS OS descriptors
        CONTROL_BUF.init([0; 64]),
    );

    let state = STATE.init(State::new());
    let class = CdcAcmClass::new(&mut builder, state, MAX_PACKET_SIZE);

    // Second CDC-ACM interface, dedicated to plain-text logs (see `crate::log`).
    let log_state = LOG_STATE.init(State::new());
    let log_class = CdcAcmClass::new(&mut builder, log_state, MAX_PACKET_SIZE);

    // Vendor "reset interface" so `picotool load -f` can reboot the board
    // into BOOTSEL mode itself, without needing the BOOT button held.
    let reset_interface_number = {
        let mut function = builder.function(reset::CLASS, reset::SUBCLASS, reset::PROTOCOL);
        let mut interface = function.interface();
        interface.alt_setting(reset::CLASS, reset::SUBCLASS, reset::PROTOCOL, None);
        interface.interface_number()
    };
    static RESET_HANDLER: StaticCell<ResetHandler> = StaticCell::new();
    builder.handler(RESET_HANDLER.init(ResetHandler::new(reset_interface_number)));

    let device = builder.build();
    let (sender, receiver) = class.split();
    let (log_sender, _log_receiver) = log_class.split();

    Usb { device, sender, receiver, log_sender }
}

#[embassy_executor::task]
pub async fn usb_task(mut device: UsbDevice<'static, UsbDriver>) -> ! {
    device.run().await
}

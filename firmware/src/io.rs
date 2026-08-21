//! GPIO pin mapping between logical IO channels and physical RP2040 pins.
//!
//! ## Outputs
//!
//! Driven through a ULN2803A Darlington array. Setting a GPIO high turns the
//! corresponding Darlington transistor on, sinking the attached load to GND
//! (the load itself is referenced to +24V). Logical channels `Out1..=Out8`
//! map directly to `GPIO0..=GPIO7`, in order, so no inversion is needed:
//! logical "on" == GPIO high.
//!
//! ## Inputs
//!
//! Sensed through PC817 optocouplers with an onboard pull-up to 3.3V. The
//! optocoupler's phototransistor sinks the GPIO low when the external input
//! is active, so the electrical level is inverted with respect to the
//! logical state reported over USB (logical/reported `1` == input active ==
//! electrical low).
//!
//! Only 7 of the 8 input channels are wired on this board revision; `In8` is
//! not connected and therefore not exposed here. The remaining channels are
//! wired to GPIOs out of numerical order:
//!
//! | Logical channel | GPIO |
//! |------------------|------|
//! | In1              | 10   |
//! | In2              | 9    |
//! | In3              | 8    |
//! | In4              | 14   |
//! | In5              | 13   |
//! | In6              | 12   |
//! | In7              | 11   |

use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::{
    PIN_0, PIN_1, PIN_2, PIN_3, PIN_4, PIN_5, PIN_6, PIN_7, PIN_8, PIN_9, PIN_10, PIN_11, PIN_12,
    PIN_13, PIN_14,
};

/// Number of digital outputs (`Out1..=Out8`).
pub const NUM_OUTPUTS: usize = 8;
/// Number of digital inputs actually wired on the board (`In1..=In7`).
pub const NUM_INPUTS: usize = 7;

/// Physical pins required to build [`Outputs`], in `Out1..=Out8` order.
pub struct OutputPins {
    pub out1: Peri<'static, PIN_0>,
    pub out2: Peri<'static, PIN_1>,
    pub out3: Peri<'static, PIN_2>,
    pub out4: Peri<'static, PIN_3>,
    pub out5: Peri<'static, PIN_4>,
    pub out6: Peri<'static, PIN_5>,
    pub out7: Peri<'static, PIN_6>,
    pub out8: Peri<'static, PIN_7>,
}

/// Physical pins required to build [`Inputs`], in `In1..=In7` order.
pub struct InputPins {
    pub in1: Peri<'static, PIN_10>,
    pub in2: Peri<'static, PIN_9>,
    pub in3: Peri<'static, PIN_8>,
    pub in4: Peri<'static, PIN_14>,
    pub in5: Peri<'static, PIN_13>,
    pub in6: Peri<'static, PIN_12>,
    pub in7: Peri<'static, PIN_11>,
}

/// All 8 digital outputs, indexed by logical channel (index 0 == `Out1`).
pub struct Outputs(pub [Output<'static>; NUM_OUTPUTS]);

impl Outputs {
    pub fn new(pins: OutputPins) -> Self {
        Self([
            Output::new(pins.out1, Level::Low),
            Output::new(pins.out2, Level::Low),
            Output::new(pins.out3, Level::Low),
            Output::new(pins.out4, Level::Low),
            Output::new(pins.out5, Level::Low),
            Output::new(pins.out6, Level::Low),
            Output::new(pins.out7, Level::Low),
            Output::new(pins.out8, Level::Low),
        ])
    }

    /// Set output `channel` (0-indexed, 0 == `Out1`) to `on`.
    ///
    /// Silently ignores out-of-range channels.
    pub fn set(&mut self, channel: u8, on: bool) {
        if let Some(pin) = self.0.get_mut(channel as usize) {
            pin.set_level(if on { Level::High } else { Level::Low });
        }
    }
}

/// The 7 wired digital inputs, indexed by logical channel (index 0 == `In1`).
pub struct Inputs(pub [Input<'static>; NUM_INPUTS]);

impl Inputs {
    pub fn new(pins: InputPins) -> Self {
        // The board provides an onboard pull-up to 3.3V for each opto input;
        // `Pull::Up` is added here too, redundantly, in case a board
        // revision omits it.
        Self([
            Input::new(pins.in1, Pull::Up),
            Input::new(pins.in2, Pull::Up),
            Input::new(pins.in3, Pull::Up),
            Input::new(pins.in4, Pull::Up),
            Input::new(pins.in5, Pull::Up),
            Input::new(pins.in6, Pull::Up),
            Input::new(pins.in7, Pull::Up),
        ])
    }
}

//! GPIO pin mapping between logical IO channels and physical RP2040 pins.
//!
//! ## Outputs
//!
//! Driven through a ULN2803A Darlington array, sinking the attached load to
//! GND (the load itself is referenced to +24V). Logical channels `Out1..=Out4`
//! map to `GPIO4..=GPIO7` and `Out5..=Out8` to `GPIO0..=GPIO3` (matching the
//! PCB silkscreen). The level is inverted: logical "on" == GPIO low, so all
//! outputs start high (off).
//!
//! ## Inputs
//!
//! Sensed through PC817 optocouplers. Measured on the board, idle inputs
//! read low, so the level is not inverted: logical/reported `1` == input
//! active == electrical high.
//!
//! Only 7 of the 8 input channels are wired as inputs; `In1` is the clock
//! and therefore not exposed here. The remaining channels are
//! wired to GPIOs out of numerical order:
//!
//! | Logical channel | GPIO |
//! |------------------|------|
//! | In2              | 10   |
//! | In3              | 9    |
//! | In4              | 8    |
//! | In5              | 14   |
//! | In6              | 13   |
//! | In7              | 12   |
//! | In8              | 11   |

use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::{
    PIN_0, PIN_1, PIN_2, PIN_3, PIN_4, PIN_5, PIN_6, PIN_7, PIN_8, PIN_9, PIN_10, PIN_11, PIN_12,
    PIN_13, PIN_14,
};

/// Number of digital outputs/inputs, re-exported from the shared wire
/// protocol crate since it also defines the `STATUS` bitmask widths.
pub use promatrix_protocol::{NUM_INPUTS, NUM_OUTPUTS};

/// Physical pins required to build [`Outputs`], in `Out1..=Out8` order.
pub struct OutputPins {
    pub out1: Peri<'static, PIN_4>,
    pub out2: Peri<'static, PIN_5>,
    pub out3: Peri<'static, PIN_6>,
    pub out4: Peri<'static, PIN_7>,
    pub out5: Peri<'static, PIN_0>,
    pub out6: Peri<'static, PIN_1>,
    pub out7: Peri<'static, PIN_2>,
    pub out8: Peri<'static, PIN_3>,
}

/// Physical pins required to build [`Inputs`], in `In2..=In8` order.
pub struct InputPins {
    pub in2: Peri<'static, PIN_10>,
    pub in3: Peri<'static, PIN_9>,
    pub in4: Peri<'static, PIN_8>,
    pub in5: Peri<'static, PIN_14>,
    pub in6: Peri<'static, PIN_13>,
    pub in7: Peri<'static, PIN_12>,
    pub in8: Peri<'static, PIN_11>,
}

/// All 8 digital outputs, indexed by logical channel (index 0 == `Out1`).
pub struct Outputs(pub [Output<'static>; NUM_OUTPUTS]);

impl Outputs {
    pub fn new(pins: OutputPins) -> Self {
        Self([
            Output::new(pins.out1, Level::High),
            Output::new(pins.out2, Level::High),
            Output::new(pins.out3, Level::High),
            Output::new(pins.out4, Level::High),
            Output::new(pins.out5, Level::High),
            Output::new(pins.out6, Level::High),
            Output::new(pins.out7, Level::High),
            Output::new(pins.out8, Level::High),
        ])
    }

    /// Set output `output` (board label number, `1..=8`) to `on`.
    ///
    /// Silently ignores out-of-range outputs.
    pub fn set(&mut self, output: u8, on: bool) {
        if let Some(pin) = self.0.get_mut((output as usize).wrapping_sub(1)) {
            pin.set_level(if on { Level::Low } else { Level::High });
        }
    }
}

/// The 7 wired digital inputs, indexed by logical channel (index 0 == `In2`).
pub struct Inputs(pub [Input<'static>; NUM_INPUTS]);

impl Inputs {
    pub fn new(pins: InputPins) -> Self {
        // NOTE: idle inputs measured low despite this pull-up; if an
        // unconnected input reads "on", switch to `Pull::Down`.
        Self([
            Input::new(pins.in2, Pull::Up),
            Input::new(pins.in3, Pull::Up),
            Input::new(pins.in4, Pull::Up),
            Input::new(pins.in5, Pull::Up),
            Input::new(pins.in6, Pull::Up),
            Input::new(pins.in7, Pull::Up),
            Input::new(pins.in8, Pull::Up),
        ])
    }
}

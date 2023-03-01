use defmt::Format;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp2040_hal::gpio::{pin, Output, PinId, Readable};

pub struct RotaryEncoder<A: PinId, B: PinId> {
    out_a: pin::Pin<A, Output<Readable>>,
    out_b: pin::Pin<B, Output<Readable>>,

    prev_state_a: bool,
}

#[derive(Format)]
pub enum RotationDirection {
    Clockwise,
    CounterClockwise,
}

impl<A: PinId, B: PinId> RotaryEncoder<A, B> {
    pub fn new(
        mut out_a: pin::Pin<A, Output<Readable>>,
        mut out_b: pin::Pin<B, Output<Readable>>,
    ) -> Self {
        out_a.set_high().unwrap();
        out_b.set_high().unwrap();

        let state_a = out_a.is_high().unwrap();

        Self {
            out_a,
            out_b,
            prev_state_a: state_a,
        }
    }

    pub fn read(&mut self) -> Option<RotationDirection> {
        let state_a = self.out_a.is_high().unwrap();
        let state_b = self.out_b.is_high().unwrap();

        if state_a == self.prev_state_a {
            return None;
        }

        self.prev_state_a = state_a;

        if state_a == state_b {
            Some(RotationDirection::CounterClockwise)
        } else {
            Some(RotationDirection::Clockwise)
        }
    }
}

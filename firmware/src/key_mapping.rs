use usbd_human_interface_device::page::Keyboard;

use crate::rotary_encoder::RotationDirection;

#[inline]
pub fn key_mapping(key_id: usize) -> Keyboard {
    match key_id {
        0 => Keyboard::A,
        1 => Keyboard::B,
        2 => Keyboard::C,
        3 => Keyboard::D,
        4 => Keyboard::E,
        _ => Keyboard::NoEventIndicated,
    }
}

#[inline]
pub fn encoder_mapping(dir: RotationDirection) -> Keyboard {
    match dir {
        RotationDirection::Clockwise => Keyboard::VolumeUp,
        RotationDirection::CounterClockwise => Keyboard::VolumeDown,
    }
}
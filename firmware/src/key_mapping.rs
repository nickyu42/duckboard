use usbd_human_interface_device::page::Keyboard;

#[inline]
pub fn key_mapping(key_id: usize) -> Keyboard {
    match key_id {
        0 => Keyboard::VolumeUp,
        1 => Keyboard::VolumeDown,
        2 => Keyboard::Mute,
        3 => Keyboard::Undo,
        4 => Keyboard::NoEventIndicated,
        _ => Keyboard::NoEventIndicated,
    }
}

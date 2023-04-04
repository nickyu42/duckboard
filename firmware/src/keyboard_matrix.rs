use cortex_m::asm;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp2040_hal::gpio::{dynpin, DynPin};
use usbd_human_interface_device::page::Keyboard;

use crate::key_mapping;

const KEY_COUNT: usize = 5;

pub struct KeyboardMatrix {
    keys: [u32; KEY_COUNT],
    key_state: [bool; KEY_COUNT],
    prev_key_state: [bool; KEY_COUNT],
    cols: [DynPin; 2],
    rows: [DynPin; 3],
}

#[derive(PartialEq)]
pub struct ScanResult {
    pub key_pressed: bool,
    pub event_occurred: bool,
}

impl ScanResult {
    pub fn should_report(&self) -> bool {
        self.key_pressed | self.event_occurred
    }
}

impl defmt::Format for KeyboardMatrix {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "KeyboardMatrix {{ {} }}", self.key_state)
    }
}

impl KeyboardMatrix {
    pub fn new(mut rows: [DynPin; 3], mut cols: [DynPin; 2]) -> Self {
        for row in &mut rows {
            row.into_pull_down_input();
        }

        for col in &mut cols {
            col.into_push_pull_output();
        }

        Self {
            keys: [0; KEY_COUNT],
            key_state: [false; KEY_COUNT],
            prev_key_state: [false; KEY_COUNT],
            cols: cols,
            rows: rows,
        }
    }

    pub fn scan(&mut self) -> Result<ScanResult, dynpin::Error> {
        let mut current_key = 0;

        let mut scan_result = ScanResult { key_pressed: false, event_occurred: false };

        self.prev_key_state = self.key_state;

        for col in &mut self.cols {
            col.set_high()?;

            asm::delay(5);

            for row in &mut self.rows {
                if current_key >= KEY_COUNT {
                    continue;
                }

                let is_pressed = row.is_high()? as u32;

                // Update key state
                self.keys[current_key] = (self.keys[current_key] << 1) | is_pressed;

                if self.key_state[current_key] && (!self.keys[current_key] & 0b1111) == 0b1111 {
                    self.key_state[current_key] = false;
                    scan_result.event_occurred = true;
                }

                if !self.key_state[current_key] && (self.keys[current_key] & 0b1111) == 0b1111 {
                    self.key_state[current_key] = true;
                    scan_result.event_occurred = true;
                }

                if self.key_state[current_key] {
                    scan_result.key_pressed = true;
                }

                current_key += 1;
            }

            col.set_low()?;
        }

        Ok(scan_result)
    }

    pub fn get_pressed_keys(&self) -> [Keyboard; KEY_COUNT] {
        let mut pressed = [Keyboard::NoEventIndicated; KEY_COUNT];

        for i in 0..KEY_COUNT {
            let curr = self.key_state[i];
            let prev = self.prev_key_state[i];
            if curr && curr != prev {
                pressed[i] = key_mapping::key_mapping(i);
            }
        }

        pressed
    } 
}

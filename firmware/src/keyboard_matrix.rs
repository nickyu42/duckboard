use cortex_m::asm;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp2040_hal::gpio::{dynpin, DynPin};

const KEY_COUNT: usize = 5;

pub struct KeyboardMatrix {
    keys: [u32; KEY_COUNT],
    key_state: [bool; KEY_COUNT],
    cols: [DynPin; 2],
    rows: [DynPin; 3],
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
            cols: cols,
            rows: rows,
        }
    }

    pub fn scan(&mut self) -> Result<(), dynpin::Error> {
        let mut current_key = 0;

        for col in &mut self.cols {
            col.set_high()?;

            asm::delay(5);

            for row in &mut self.rows {
                if current_key >= KEY_COUNT {
                    continue;
                }

                let is_pressed = row.is_high()?;

                // Update key state
                self.keys[current_key] = (self.keys[current_key] << 1) | (is_pressed as u32);

                if self.key_state[current_key] && (!self.keys[current_key] & 0b1111) == 0b1111 {
                    self.key_state[current_key] = false;
                }

                if !self.key_state[current_key] && (self.keys[current_key] & 0b1111) == 0b1111 {
                    self.key_state[current_key] = true;
                }

                current_key += 1;
            }

            col.set_low()?;
        }

        Ok(())
    }
}

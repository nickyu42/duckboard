use rp2040_hal as hal;

#[link_section = ".boot2"]
#[no_mangle]
#[used]
static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

hal::bsp_pins!(
    Gpio1 { name: row0 },
    Gpio2 { name: col1 },
    Gpio3 { name: row1 },
    Gpio4 { name: col0 },
    Gpio18 { name: rot1 },
    Gpio19 { name: rot0 },
    Gpio20 { name: row2 },
    Gpio21 { name: led_out },
);

pub const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

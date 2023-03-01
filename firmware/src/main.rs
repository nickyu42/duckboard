#![no_std]
#![no_main]

// Entry point
use cortex_m_rt::entry;

// Setup defmt with RTT
use defmt::*;
use defmt_rtt as _;

// Handle panicking with probe-run
use panic_probe as _;

// Setup hal
use rp2040_hal as hal;

use embedded_hal::digital::v2::{InputPin, OutputPin};

use hal::{
    clocks::init_clocks_and_plls,
    gpio::{dynpin, pin, DynPin, Output, PinId, Readable},
    pac,
    sio::Sio,
    timer::Timer,
    usb::UsbBus,
    watchdog::Watchdog,
    Clock,
};

use usb_device::{class_prelude::*, prelude::*};
use usbd_hid::{
    descriptor::{KeyboardReport, SerializedDescriptor},
    hid_class::{
        HIDClass, HidClassSettings, HidCountryCode, HidProtocol, HidSubClass, ProtocolModeConfig,
    },
};

mod board_config;
mod keyboard_matrix;
mod rotary_encoder;

use keyboard_matrix::KeyboardMatrix;
use rotary_encoder::RotaryEncoder;

#[entry]
fn main() -> ! {
    info!("Hello from duckboard");

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let clocks = init_clocks_and_plls(
        board_config::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = board_config::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut matrix = KeyboardMatrix::new(
        [pins.row0.into(), pins.row1.into(), pins.row2.into()],
        [pins.col0.into(), pins.col1.into()],
    );

    let mut encoder = RotaryEncoder::new(
        pins.rot0.into_readable_output(),
        pins.rot1.into_readable_output(),
    );

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut hid = HIDClass::new_with_settings(
        &usb_bus,
        KeyboardReport::desc(),
        10,
        HidClassSettings {
            subclass: HidSubClass::NoSubClass,
            protocol: HidProtocol::Keyboard,
            config: ProtocolModeConfig::ForceReport,
            locale: HidCountryCode::NotSupported,
        },
    );

    let vid_pid = UsbVidPid(0x16c0, 0x27dd);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, vid_pid)
        .manufacturer("Compubotics")
        .product("Duckboard")
        .serial_number("4242")
        .max_packet_size_0(64)
        .device_class(2)
        .build();

    loop {
        usb_dev.poll(&mut [&mut hid]);

        matrix.scan().unwrap();

        if let Some(r) = encoder.read() {
            debug!("{:?}", r);
        }
        // debug!("{:?} {:?}", matrix, encoder.read());
        delay.delay_us(1);

        // hid.pull_raw_output(&mut [0; 64]).ok();
    }

    // let timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    // let usb_bus = UsbBusAllocator::new(UsbBus::new(
    //     pac.USBCTRL_REGS,
    //     pac.USBCTRL_DPRAM,
    //     clocks.usb_clock,
    //     true,
    //     &mut pac.RESETS,
    // ));
    // let mut serial = SerialPort::new(&usb_bus);
    // let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
    //     .manufacturer("Fake company")
    //     .product("Serial port")
    //     .serial_number("TEST")
    //     .max_packet_size_0(64)
    //     .device_class(2) // from: https://www.usb.org/defined-class-codes
    //     .build();

    // let mut said_hello = false;
    // loop {
    //     if !said_hello && timer.get_counter() >= 2_000_000 {
    //         said_hello = true;
    //         let _ = serial.write(b"HelloWorld!\r\n");
    //     }

    //     if !usb_dev.poll(&mut [&mut serial]) {
    //         continue;
    //     }

    //     let mut buf = [0u8; 64];
    //     let _ = serial.read(&mut buf).map(|count| {
    //         if count == 0 {
    //             return;
    //         }

    //         // Echo back in upper case
    //         buf.iter_mut().take(count).for_each(|c| {
    //             if let 0x61..=0x7a = *c {
    //                 *c &= !0x20;
    //             }
    //         });

    //         let mut wr_ptr = &buf[..count];
    //         while !wr_ptr.is_empty() {
    //             let _ = serial.write(wr_ptr).map(|len| {
    //                 wr_ptr = &wr_ptr[len..];
    //             });
    //         }
    //     });
    // }
}

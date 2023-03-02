#![no_std]
#![no_main]

// use cortex_m::prelude::*;

// Handle panicking with probe-run
use panic_probe as _;

// Setup hal
// use rp2040_hal as hal;

use rtic;

// use embedded_hal::digital::v2::{InputPin, OutputPin};

// use hal::{
//     clocks::init_clocks_and_plls,
//     gpio::{dynpin, pin, DynPin, Output, PinId, Readable},
//     pac,
//     sio::Sio,
//     timer::Timer,
//     usb::UsbBus,
//     watchdog::Watchdog,
//     Clock,
// };

// use usb_device::{class_prelude::*, prelude::*};
// use usbd_hid::{
//     descriptor::{KeyboardReport, SerializedDescriptor},
//     hid_class::{
//         HIDClass, HidClassSettings, HidCountryCode, HidProtocol, HidSubClass, ProtocolModeConfig,
//     },
// };

mod board_config;
mod keyboard_matrix;
mod rotary_encoder;

// use keyboard_matrix::KeyboardMatrix;
// use rotary_encoder::RotaryEncoder;

#[rtic::app(device = rp2040_hal::pac, peripherals = true, dispatchers = [TIMER_IRQ_1])]
mod app {
    use rp2040_hal as hal;

    // Setup defmt with RTT
    use defmt::*;
    use defmt_rtt as _;

    // use fugit::{ExtU32};

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

    use rp2040_hal::timer::Alarm0;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type MyMono = rp2040_hal::timer::monotonic::Monotonic<Alarm0>;

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        info!("Hello from duckboard");

        let mut watchdog = Watchdog::new(cx.device.WATCHDOG);
        let sio = Sio::new(cx.device.SIO);

        let clocks = init_clocks_and_plls(
            super::board_config::XOSC_CRYSTAL_FREQ,
            cx.device.XOSC,
            cx.device.CLOCKS,
            cx.device.PLL_SYS,
            cx.device.PLL_USB,
            &mut cx.device.RESETS,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let mut timer = hal::Timer::new(cx.device.TIMER, &mut cx.device.RESETS);
        let alarm = timer.alarm_0().unwrap();

        let mono = hal::timer::monotonic::Monotonic::new(timer, alarm);

        (Shared {}, Local {}, init::Monotonics(mono))
    }

    #[task]
    fn foo(_: foo::Context) {
        info!("foo");
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }
}

// #[entry]
// fn main() -> ! {
//     info!("Hello from duckboard");

//     let mut pac = pac::Peripherals::take().unwrap();
//     let core = pac::CorePeripherals::take().unwrap();
//     let mut watchdog = Watchdog::new(pac.WATCHDOG);
//     let sio = Sio::new(pac.SIO);

//     let clocks = init_clocks_and_plls(
//         board_config::XOSC_CRYSTAL_FREQ,
//         pac.XOSC,
//         pac.CLOCKS,
//         pac.PLL_SYS,
//         pac.PLL_USB,
//         &mut pac.RESETS,
//         &mut watchdog,
//     )
//     .ok()
//     .unwrap();

//     let pins = board_config::Pins::new(
//         pac.IO_BANK0,
//         pac.PADS_BANK0,
//         sio.gpio_bank0,
//         &mut pac.RESETS,
//     );

//     let mut matrix = KeyboardMatrix::new(
//         [pins.row0.into(), pins.row1.into(), pins.row2.into()],
//         [pins.col0.into(), pins.col1.into()],
//     );

//     let mut encoder = RotaryEncoder::new(
//         pins.rot0.into_readable_output(),
//         pins.rot1.into_readable_output(),
//     );

//     let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

//     let usb_bus = UsbBusAllocator::new(UsbBus::new(
//         pac.USBCTRL_REGS,
//         pac.USBCTRL_DPRAM,
//         clocks.usb_clock,
//         true,
//         &mut pac.RESETS,
//     ));

//     let mut hid = HIDClass::new_with_settings(
//         &usb_bus,
//         KeyboardReport::desc(),
//         10,
//         HidClassSettings {
//             subclass: HidSubClass::NoSubClass,
//             protocol: HidProtocol::Keyboard,
//             config: ProtocolModeConfig::ForceReport,
//             locale: HidCountryCode::NotSupported,
//         },
//     );

//     let vid_pid = UsbVidPid(0x16c0, 0x27dd);
//     let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, vid_pid)
//         .manufacturer("Compubotics")
//         .product("Duckboard")
//         .max_packet_size_0(64)
//         .device_class(2)
//         .build();

//     loop {
//         usb_dev.poll(&mut [&mut hid]);

//         matrix.scan().unwrap();

//         if let Some(r) = encoder.read() {
//             debug!("{:?}", r);
//         }

//         hid.pull_raw_output(&mut [0; 64]).ok();
//     }
// }

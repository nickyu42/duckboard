#![no_std]
#![no_main]

// Handle panicking with probe-run
use panic_probe as _;

use rtic;

mod board_config;
mod keyboard_matrix;
mod rotary_encoder;

#[rtic::app(device = rp2040_hal::pac, peripherals = true, dispatchers = [TIMER_IRQ_1])]
mod app {
    use rp2040_hal::{self as hal, usb::UsbBus};

    // Setup defmt with RTT
    use defmt::*;
    use defmt_rtt as _;

    use fugit::ExtU32;

    use hal::{clocks::init_clocks_and_plls, sio::Sio, watchdog::Watchdog};

    use rp2040_hal::timer::Alarm0;

    use usb_device::class_prelude::UsbBusAllocator;
    use usb_device::prelude::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
    use usbd_hid::descriptor::KeyboardReport;
    use usbd_hid::descriptor::SerializedDescriptor;
    use usbd_hid::hid_class;

    use crate::{board_config, keyboard_matrix, rotary_encoder};

    #[shared]
    struct Shared {
        hid: hid_class::HIDClass<'static, UsbBus>,
    }

    #[local]
    struct Local {
        matrix: keyboard_matrix::KeyboardMatrix,
        encoder: rotary_encoder::RotaryEncoder<hal::gpio::bank0::Gpio19, hal::gpio::bank0::Gpio18>,
        usb_dev: UsbDevice<'static, UsbBus>,
    }

    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type MyMono = rp2040_hal::timer::monotonic::Monotonic<Alarm0>;

    #[init(local = [usb_bus: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None])]
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

        info!("Setting up peripherals");

        // Setup input peripherals
        let pins = board_config::Pins::new(
            cx.device.IO_BANK0,
            cx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut cx.device.RESETS,
        );

        let matrix = keyboard_matrix::KeyboardMatrix::new(
            [pins.row0.into(), pins.row1.into(), pins.row2.into()],
            [pins.col0.into(), pins.col1.into()],
        );

        let encoder = rotary_encoder::RotaryEncoder::new(
            pins.rot0.into_readable_output(),
            pins.rot1.into_readable_output(),
        );

        info!("Setting up USB");

        // Setup USB device and HID handler
        let usb_bus: &'static _ = cx.local.usb_bus.insert(UsbBusAllocator::new(UsbBus::new(
            cx.device.USBCTRL_REGS,
            cx.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut cx.device.RESETS,
        )));

        let hid = hid_class::HIDClass::new_with_settings(
            &usb_bus,
            KeyboardReport::desc(),
            10,
            hid_class::HidClassSettings {
                subclass: hid_class::HidSubClass::NoSubClass,
                protocol: hid_class::HidProtocol::Keyboard,
                config: hid_class::ProtocolModeConfig::ForceReport,
                locale: hid_class::HidCountryCode::NotSupported,
            },
        );

        let vid_pid = UsbVidPid(0x16c0, 0x27dd);
        let usb_dev = UsbDeviceBuilder::new(&usb_bus, vid_pid)
            .manufacturer("Compubotics")
            .product("Duckboard")
            .serial_number("4242")
            .max_packet_size_0(64)
            .device_class(2)
            .build();

        info!("Spawning monotonic tasks");

        // Setup timer for tasks
        let mut timer = hal::Timer::new(cx.device.TIMER, &mut cx.device.RESETS);
        let alarm = timer.alarm_0().unwrap();
        let mono = hal::timer::monotonic::Monotonic::new(timer, alarm);

        // Spawn tasks
        usb_poll::spawn().unwrap();
        poll_inputs::spawn().unwrap();

        info!("Init done! :)");

        (
            Shared { hid },
            Local {
                matrix,
                encoder,
                usb_dev,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [encoder, matrix], shared = [hid])]
    fn poll_inputs(mut cx: poll_inputs::Context) {
        if cx.local.matrix.scan().unwrap() {
            debug!("{:?}", cx.local.matrix);
        };

        if let Some(r) = cx.local.encoder.read() {
            debug!("{:?}", r);

            let report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0x0e, 0, 0, 0, 0, 0],
            };

            cx.shared.hid.lock(|hid| {
                hid.push_input(&report).unwrap();
            });
        }

        poll_inputs::spawn_at(monotonics::now() + 5_u32.millis()).unwrap();
    }

    #[task(local = [usb_dev], shared = [hid])]
    fn usb_poll(mut cx: usb_poll::Context) {
        // debug!("poll");
        cx.shared.hid.lock(|hid| {
            // Poll USB device
            cx.local.usb_dev.poll(&mut [hid]);

            // clear HID
            hid.pull_raw_output(&mut [0; 64]).ok();
        });

        usb_poll::spawn_at(monotonics::now() + 10_u32.millis()).unwrap();
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

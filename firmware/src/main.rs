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

    use hal::{clocks::init_clocks_and_plls, pio::PIOExt, sio::Sio, watchdog::Watchdog, Clock};

    use rp2040_hal::timer::Alarm0;

    use usb_device::class_prelude::UsbBusAllocator;
    use usb_device::prelude::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
    // use usbd_hid::descriptor::{KeyboardReport, MediaKeyboardReport, MediaKey};
    // use usbd_hid::descriptor::SerializedDescriptor;
    // use usbd_hid::hid_class;

    use usbd_human_interface_device::device::keyboard::{
        KeyboardLedsReport, NKROBootKeyboardInterface,
    };
    use usbd_human_interface_device::page::Keyboard;
    use usbd_human_interface_device::prelude::*;

    use ws2812_pio::Ws2812Direct;
    use smart_leds::{SmartLedsWrite, RGB8};

    use crate::{board_config, keyboard_matrix, rotary_encoder};

    #[shared]
    struct Shared {
        // hid: hid_class::HIDClass<'static, UsbBus>,
        hid: UsbHidClass<UsbBus, frunk::HList!(NKROBootKeyboardInterface<'static, UsbBus>)>,

        ws: Ws2812Direct<hal::pac::PIO0, hal::pio::SM0, hal::gpio::bank0::Gpio21>,
    }

    #[local]
    struct Local {
        matrix: keyboard_matrix::KeyboardMatrix,
        encoder: rotary_encoder::RotaryEncoder<hal::gpio::bank0::Gpio19, hal::gpio::bank0::Gpio18>,
        usb_dev: UsbDevice<'static, UsbBus>,
    }

    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type Monotonic = rp2040_hal::timer::monotonic::Monotonic<Alarm0>;

    #[init(local = [usb_bus: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None])]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Soft-reset does not release the hardware spinlocks
        // Release them now to avoid a deadlock after debug or watchdog reset
        unsafe {
            hal::sio::spinlock_reset();
        }

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

        let (mut pio, sm0, _, _, _) = cx.device.PIO0.split(&mut cx.device.RESETS);
        let mut ws = Ws2812Direct::new(
            pins.led_out.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
        );

        // let color : RGB8 = (100, 0, 0).into();
        // ws.write([color, color, color].iter().copied()).unwrap();

        info!("Setting up USB");

        // Setup USB device and HID handler
        let usb_bus: &'static _ = cx.local.usb_bus.insert(UsbBusAllocator::new(UsbBus::new(
            cx.device.USBCTRL_REGS,
            cx.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut cx.device.RESETS,
        )));

        let mut hid = UsbHidClassBuilder::new()
            .add_interface(NKROBootKeyboardInterface::default_config())
            .build(&usb_bus);

        let vid_pid = UsbVidPid(0x1209, 0x0001);
        let usb_dev = UsbDeviceBuilder::new(&usb_bus, vid_pid)
            .manufacturer("Compubotics")
            .product("Duckboard")
            .serial_number("4242")
            .build();

        info!("Spawning monotonic tasks");

        // Setup timer for tasks
        let mut timer = hal::Timer::new(cx.device.TIMER, &mut cx.device.RESETS);
        let alarm = timer.alarm_0().unwrap();
        let mono = hal::timer::monotonic::Monotonic::new(timer, alarm);

        // Enable the USB interrupt
        // unsafe {
        //     hal::pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        // };

        // Spawn tasks
        // usb_poll::spawn().unwrap();
        poll_inputs::spawn().unwrap();
        tick::spawn().unwrap();

        info!("Init done! :)");

        (
            Shared { hid, ws },
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

            // let report = MediaKeyboardReport {
            //     usage_id: MediaKey::VolumeIncrement.into(),
            // };

            cx.shared.hid.lock(|hid| {
                hid.interface().write_report([Keyboard::A]);
            });

            // cortex_m::interrupt::free(|_| {
            //     cx.shared.hid.lock(|hid| {
            //         info!("{:?}", hid.push_input(&report));
            //     });
            // })
        } else {
            cx.shared.hid.lock(|hid| {
                hid.interface().write_report([Keyboard::NoEventIndicated]);
            });
        }

        // cx.shared.hid.lock(|hid| {
        //     hid.interface().tick().unwrap();
        // });

        poll_inputs::spawn_at(monotonics::now() + 5_u32.millis()).unwrap();
    }

    #[task(shared = [hid])]
    fn tick(mut cx: tick::Context) {
        cx.shared.hid.lock(|k| match k.interface().tick() {
            Err(UsbHidError::WouldBlock) => {}
            Ok(_) => {}
            Err(e) => {
                core::panic!("Failed to process hid tick: {:?}", e)
            }
        });

        tick::spawn_at(monotonics::now() + 1_u32.millis()).unwrap();
    }

    #[task(binds = USBCTRL_IRQ, priority = 3, local = [usb_dev], shared = [hid])]
    fn usb_rx(mut cx: usb_rx::Context) {
        cx.shared.hid.lock(|hid| {
            // Poll USB device
            if cx.local.usb_dev.poll(&mut [hid]) {
                hid.interface().read_report().ok();
            }
        });
    }

    // #[task(local = [usb_dev], shared = [hid])]
    // fn usb_poll(mut cx: usb_poll::Context) {
    //     // debug!("poll");
    //     cx.shared.hid.lock(|hid| {
    //         // Poll USB device
    //         cx.local.usb_dev.poll(&mut [hid]);

    //         // clear HID
    //         hid.pull_raw_output(&mut [0; 64]).ok();
    //     });

    //     usb_poll::spawn_at(monotonics::now() + 10_u32.millis()).unwrap();
    // }

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

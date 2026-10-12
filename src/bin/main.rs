#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use alloc::vec;
use core::cell::RefCell;
use defmt;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    prelude::*,
    text::Text,
};
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
use esp_hal::spi::Mode as SpiMode;
use esp_hal::spi::master::Config as SpiConfig;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use ssd1677::{Builder, Color, Dimensions, Display, GraphicDisplay, Interface, Rotation};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const WIDTH: u16 = 528;
const HEIGHT: u16 = 792;
const BUFFER_SIZE: u16 = 52272;

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.4.0
    // generator parameters: -o esp32c3 -o unstable-hal -o embassy -o ci -o zed -o alloc -o wifi -o stable-x86_64-unknown-linux-gnu

    defmt::info!("starting up");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    defmt::info!("initializing SPI");
    // Initialize SPI
    let spi_bus = RefCell::new(
        Spi::new(
            peripherals.SPI2,
            SpiConfig::default()
                .with_frequency(Rate::from_mhz(60))
                .with_mode(SpiMode::_0),
        )
        .unwrap()
        //CLK
        .with_sck(peripherals.GPIO8)
        //DIN
        .with_mosi(peripherals.GPIO10),
    );

    let d = Delay::new();

    let cs = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());
    let spi = RefCellDevice::new(&spi_bus, cs, d).expect("Could not create SPI");

    let dc = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());
    let busy = Input::new(peripherals.GPIO6, InputConfig::default());

    let interface = Interface::new(spi, dc, rst, busy);

    defmt::info!("initializing display");
    let dims = Dimensions::new(WIDTH, HEIGHT).expect("Could not set display dimensions");

    let display_config = Builder::new()
        .dimensions(dims)
        .rotation(Rotation::Rotate0)
        .build()
        .expect("Could not build display");

    let display = Display::new(interface, display_config);
    let mut delay = Delay::new();

    defmt::info!("2");
    let mut graphic_display = GraphicDisplay::new(
        display,
        vec![0u8; BUFFER_SIZE.into()], // Black buffer
        vec![0u8; BUFFER_SIZE.into()], // Red buffer
    );
    defmt::info!("3");
    Text::new(
        "Hello, E-Paper!",
        Point::new(10, 20),
        MonoTextStyle::new(&FONT_6X10, Color::Black),
    )
    .draw(&mut graphic_display)
    .expect("Could not draw graphics");

    defmt::info!("4");
    // Update display
    graphic_display
        .update(&mut delay)
        .unwrap_or(defmt::error!("Could not update display"));

    defmt::info!("initializing wifi");
    let _wifi_controller =
        esp_radio::wifi::WifiController::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    let _wifi_interface = esp_radio::wifi::Interface::station();

    // TODO: Spawn some tasks
    let _ = spawner;

    defmt::info!("running main loop");
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.2.2/examples
}

#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Output, OutputConfig};
use esp_hal::main;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::{Duration, Instant};
use esp_println::println;
use tropic01::Tropic01;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // PINS
    // Tropic01 <=> ESP32
    // CS <=> FSPICS0 (GPIO16)
    // SCK <=> FSPICLK (GPIO6)
    // SDO <=> FSPIQ/MISO (GPIO2)
    // SDI <=> FSPID/MOSI (GPIO7)
    // 3.3V <=> 3.3V
    // GND <=> GND
    let spi_bus = Spi::new(peripherals.SPI2, Config::default())
        .unwrap()
        //.with_cs(peripherals.GPIO16)
        .with_sck(peripherals.GPIO6)
        .with_miso(peripherals.GPIO2)
        .with_mosi(peripherals.GPIO7);

    let cs = Output::new(
        peripherals.GPIO16,
        esp_hal::gpio::Level::High,
        OutputConfig::default(),
    );

    let delay = Delay::new();

    let spi = ExclusiveDevice::new(spi_bus, cs, delay).unwrap();

    let mut tropic = Tropic01::new(spi);

    let chip_id = tropic.get_info_chip_id();

    if let Ok(chip_id) = chip_id {
        println!("Chip ID: {:?}", chip_id);
    }

    println!("Sleeping...");
    loop {
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
    }
}

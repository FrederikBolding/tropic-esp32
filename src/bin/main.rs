#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use ed25519_dalek::{Signature, VerifyingKey};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Output, OutputConfig};
use esp_hal::main;
use esp_hal::rng::{Trng, TrngSource};
use esp_hal::sha::{Sha, Sha256, ShaAlgorithm};
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::{Duration, Instant};
use esp_println::println;
use tropic01::{Tropic01, X25519Dalek};
use x25519_dalek::{PublicKey, StaticSecret};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const SH0PRIV: [u8; 32] = [
    0x28, 0x3f, 0x5a, 0x0f, 0xfc, 0x41, 0xcf, 0x50, 0x98, 0xa8, 0xe1, 0x7d, 0xb6, 0x37, 0x2c, 0x3c,
    0xaa, 0xd1, 0xee, 0xee, 0xdf, 0x0f, 0x75, 0xbc, 0x3f, 0xbf, 0xcd, 0x9c, 0xab, 0x3d, 0xe9, 0x72,
];
const SH0PUB: [u8; 32] = [
    0xf9, 0x75, 0xeb, 0x3c, 0x2f, 0xd7, 0x90, 0xc9, 0x6f, 0x29, 0x4f, 0x15, 0x57, 0xa5, 0x03, 0x17,
    0x80, 0xc9, 0xaa, 0xfa, 0x14, 0x0d, 0xa2, 0x8f, 0x55, 0xe7, 0x51, 0x57, 0x37, 0xb2, 0x50, 0x2c,
];

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

    let chip_id = tropic.get_info_chip_id().unwrap();

    println!("Chip ID: {:?}", chip_id);

    let fw_version = tropic.get_info_riscv_fw_ver().unwrap();
    println!("Firmware: {:?}", fw_version);

    println!("Setting up session...");

    // The side-effect of creating TrngSource allows the creation of Trng
    let _trng_source = TrngSource::new(peripherals.RNG, peripherals.ADC1);
    let rng = Trng::try_new().unwrap();

    let ephemeral_key = StaticSecret::random_from_rng(rng);
    let ephemeral_pub_key = PublicKey::from(&ephemeral_key);

    tropic
        .session_start(
            &X25519Dalek,
            SH0PUB.into(),
            SH0PRIV.into(),
            ephemeral_pub_key,
            ephemeral_key,
            0,
        )
        .unwrap();

    let random_number = tropic.get_random_value(1).unwrap();
    println!("Dice Roll: {}", random_number[0]);

    let key_slot = 0.into();
    // tropic.ecc_key_generate(key_slot, tropic01::EccCurve::Ed25519).unwrap();

    let key_read = tropic.ecc_key_read(key_slot).unwrap();
    let pub_key = VerifyingKey::from_bytes(key_read.pub_key().try_into().unwrap()).unwrap();

    let mut sha_driver = Sha::new(peripherals.SHA);

    let mut digest = sha_driver.start::<Sha256>();
    digest.update("Hello world!".as_bytes()).unwrap();

    let mut hash = [0; Sha256::DIGEST_LENGTH];
    digest.finish(&mut hash).unwrap();

    let signature = tropic.eddsa_sign(key_slot, &hash).unwrap();

    println!("Signature: {:?}", signature);

    match pub_key.verify_strict(&hash, &Signature::from_bytes(signature)) {
        Ok(_) => println!("Signature valid!"),
        Err(_) => println!("Signature invalid!"),
    }

    println!("Sleeping...");
    loop {
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
    }
}

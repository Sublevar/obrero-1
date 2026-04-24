use embedded_hal::spi::SpiDevice as _;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::{
    config::Config as SpiConfig, Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig,
};
use esp_idf_svc::hal::units::Hertz;

use crate::perifericos::{update_encoders_from_bits, Encoder};

pub unsafe extern "C" fn lectoras(_: *mut core::ffi::c_void) {
    let peripherals = match Peripherals::take() {
        Ok(p) => p,
        Err(e) => {
            println!("[lectoras] ERROR: no se pudo tomar Peripherals: {:?}", e);
            return;
        }
    };

    // LC / SH-!LD: pulso LOW → latch entradas paralelas, luego HIGH
    let mut lc = PinDriver::output(peripherals.pins.gpio22).unwrap();
    lc.set_high().unwrap();

    // SPI2: SCLK=gpio23, MOSI=gpio2 (dummy), MISO=gpio35 (QH salida serie)
    let driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio23,       // SCLK → CLK
        peripherals.pins.gpio2,        // MOSI → no conectado
        Some(peripherals.pins.gpio35), // MISO ← QH
        &SpiDriverConfig::new().dma(Dma::Auto(32)),
    )
    .unwrap();

    // Modo 0 (CPOL=0 / CPHA=0): CLK idle LOW, muestreo en flanco ascendente
    let spi_config = SpiConfig::new().baudrate(Hertz(1_000_000));
    let mut spi =
        SpiDeviceDriver::new(driver, Option::<AnyIOPin>::None, &spi_config).unwrap();

    println!("[lectoras] SPI inicializado, entrando al loop");

    let mut encoders: [Encoder; 8] = core::array::from_fn(|_| Encoder::new());
    let mut ciclo: u32 = 0;

    loop {
        ciclo += 1;

        // Pulso LOW en LC: latch simultáneo de los 3 chips
        lc.set_low().unwrap();
        lc.set_high().unwrap();

        // Leer 3 bytes via SPI/DMA (24 pulsos CLK)
        let mut buf = [0u8; 3];
        match spi.read(&mut buf) {
            Ok(_) => {
                let bits: u32 =
                    ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);

                println!(
                    "[spi] chip3: 0x{:02X} ({:08b})  chip2: 0x{:02X} ({:08b})  chip1: 0x{:02X} ({:08b})  | 24-bit: {:024b}",
                    buf[0], buf[0], buf[1], buf[1], buf[2], buf[2], bits,
                );

                update_encoders_from_bits(&mut encoders, bits);
            }
            Err(e) => println!("[spi] error: {:?}", e),
        }

        if ciclo % 15 == 0 {
            println!("[tick] alive - ciclo {}", ciclo);
        }

        FreeRtos::delay_ms(20);
    }
}

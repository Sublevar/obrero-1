use embedded_hal::spi::SpiDevice as _;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::{
    config::Config as SpiConfig, Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig,
};
use esp_idf_svc::hal::units::Hertz;

pub unsafe extern "C" fn lectoras(_: *mut core::ffi::c_void) {
    let peripherals = Peripherals::take().unwrap();

    // LC / SH-!LD: pulso LOW → latch entradas paralelas, luego HIGH
    // ESP32-S3: se propone `gpio10` para PL (LC)
    let mut lc = PinDriver::output(peripherals.pins.gpio10).unwrap();
    lc.set_high().unwrap();

    // SPI: SCLK=gpio18, MOSI=gpio19 (dummy), MISO=gpio20 (Q7 salida serie) — mapeo ESP32-S3
    let driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio18,       // SCLK → CLK
        peripherals.pins.gpio19,        // MOSI → no conectado
        Some(peripherals.pins.gpio20), // MISO ← QH
        &SpiDriverConfig::new().dma(Dma::Auto(32)),
    )
    .unwrap();

    let spi_config = SpiConfig::new().baudrate(Hertz(1_000_000));
    let mut spi =
        SpiDeviceDriver::new(driver, Option::<AnyIOPin>::None, &spi_config).unwrap();

    let mut ultimo: u16 = 0xFFFF;

    loop {
        // Pulso LOW en LC: latch entradas paralelas
        lc.set_low().unwrap();
        lc.set_high().unwrap();

        // Leer 2 bytes via SPI (16 pulsos CLK) — dos 74HC165 en cascada
        let mut buf = [0u8; 2];
        if spi.read(&mut buf).is_ok() {
            let bits = u16::from_be_bytes(buf);
            if bits != ultimo {
                ultimo = bits;
                println!("[spi] {:016b}", bits);
            }
        }

        FreeRtos::delay_ms(10);
    }
}

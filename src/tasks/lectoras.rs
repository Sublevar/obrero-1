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

    let spi_config = SpiConfig::new().baudrate(Hertz(1_000_000));
    let mut spi =
        SpiDeviceDriver::new(driver, Option::<AnyIOPin>::None, &spi_config).unwrap();

    let mut ultimo: u8 = 0xFF;

    loop {
        // Pulso LOW en LC: latch entradas paralelas
        lc.set_low().unwrap();
        lc.set_high().unwrap();

        // Leer 1 byte via SPI (8 pulsos CLK) — un 74HC165
        let mut buf = [0u8; 1];
        if spi.read(&mut buf).is_ok() {
            let bits = buf[0];
            if bits != ultimo {
                ultimo = bits;
                println!("[spi] {:08b}", bits);
            }
        }

        FreeRtos::delay_ms(10);
    }
}

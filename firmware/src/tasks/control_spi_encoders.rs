use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::{
    config::Config as SpiConfig, Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig,
};
use esp_idf_svc::hal::units::Hertz;

pub unsafe extern "C" fn control_spi_encoders(_: *mut core::ffi::c_void) {
    let peripherals = Peripherals::take().unwrap();

    // LC / SH-!LD: pulso LOW → latch entradas paralelas, luego HIGH
    // ESP32-S3: gpio10 → PL (LC)
    let mut lc = PinDriver::output(peripherals.pins.gpio10).unwrap();
    lc.set_high().unwrap();

    // SPI: SCLK=gpio18, MOSI=gpio4 (dummy/DS no conectado), MISO=gpio5 (Q7 salida serie) — mapeo ESP32-S3
    // NOTA: gpio19/gpio20 son USB D-/D+ y NO deben usarse si se necesita serial USB-CDC
    let driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio18,      // SCLK → CP
        peripherals.pins.gpio4,       // MOSI → DS (no conectado, dummy)
        Some(peripherals.pins.gpio5), // MISO ← Q7 (salida serie del último 74HC165)
        &SpiDriverConfig::new().dma(Dma::Auto(32)),
    )
    .unwrap();

    let spi_config = SpiConfig::new().baudrate(Hertz(1_000_000));
    let mut spi =
        SpiDeviceDriver::new(driver, Option::<AnyIOPin>::None, &spi_config).unwrap();

    loop {
        // Pulso LOW en LC: latch entradas paralelas
        lc.set_low().unwrap();
        lc.set_high().unwrap();

        // Leer 2 bytes via SPI (16 pulsos CLK) — dos 74HC165 en cascada
        let mut buf = [0u8; 2];
        match spi.read(&mut buf) {
            Ok(_) => {
                let bits = u16::from_be_bytes(buf);
                println!("[spi] {:016b}", bits);
            }
            Err(e) => {
                println!("[spi] error de lectura: {:?}", e);
            }
        }

        FreeRtos::delay_ms(100);
    }
}

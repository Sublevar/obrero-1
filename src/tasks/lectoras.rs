use embedded_hal::spi::SpiDevice as _;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::{
    config::Config as SpiConfig, Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig,
};
use esp_idf_svc::hal::units::Hertz;

const DEBUG: bool = false;

pub unsafe extern "C" fn lectoras(_: *mut core::ffi::c_void) {
    let peripherals = match Peripherals::take() {
        Ok(p) => p,
        Err(e) => {
            if DEBUG {
                println!("[lectoras] ERROR: no se pudo tomar Peripherals: {:?}", e);
            }
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

    if DEBUG {
        println!("[lectoras] SPI inicializado, entrando al loop");
    }

    let mut ultimo: u16 = 0xFFFF;

    loop {
        // Pulso LOW en LC: latch simultáneo de los 2 chips en cascada
        lc.set_low().unwrap();
        lc.set_high().unwrap();

        // Leer 2 bytes via SPI (16 pulsos CLK) — dos 74HC165 en cascada
        let mut buf = [0u8; 2];
        match spi.read(&mut buf) {
            Ok(_) => {
                let bits: u16 = ((buf[0] as u16) << 8) | (buf[1] as u16);

                if bits != ultimo {
                    ultimo = bits;
                    println!(
                        "[spi] {:04b} {:04b} {:04b} {:04b}",
                        (bits >> 12) & 0xF,
                        (bits >> 8) & 0xF,
                        (bits >> 4) & 0xF,
                        bits & 0xF,
                    );
                }

                if DEBUG {
                    println!("[spi] buf = {:08b} {:08b}", buf[0], buf[1]);
                }
            }
            Err(e) => {
                if DEBUG {
                    println!("[spi] error: {:?}", e);
                }
            }
        }

        //FreeRtos::delay_ms(20);
    }
}

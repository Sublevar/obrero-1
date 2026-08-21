#[cfg(feature = "logging-encoders-raw")]
pub mod control_spi_encoders;
#[cfg(feature = "logging-encoders-raw")]
pub use control_spi_encoders::control_spi_encoders;

pub mod reloj_debug;

#[cfg(feature = "logging-encoders-raw")]
pub mod control_spi_encoders;
#[cfg(feature = "logging-encoders-raw")]
pub use control_spi_encoders::control_spi_encoders;

pub mod task1;
pub use task1::task1;
pub mod task2;
pub mod task3;

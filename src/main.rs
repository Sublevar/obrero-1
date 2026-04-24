mod perifericos;
mod tasks;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::sys::xTaskCreatePinnedToCore;
use std::ffi::CString;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();

    unsafe {
        xTaskCreatePinnedToCore(
            Some(tasks::lectoras),
            CString::new("lectoras").unwrap().as_ptr(),
            8192,
            std::ptr::null_mut(),
            5,
            std::ptr::null_mut(),
            0,
        );
    }

    loop {
        FreeRtos::delay_ms(30_000);
        println!("[main] alive");
    }
}
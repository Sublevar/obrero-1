mod perifericos;
mod tasks;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::sys::xTaskCreatePinnedToCore;
use std::ffi::CString;
fn main() -> anyhow::Result<()> {
    //codigo que partchea RUST para operar en ESP32
    esp_idf_svc::sys::link_patches();

    //https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/freertos_idf.html#tasks
    //xTaskCreatePinnedToCore(TaskFunction_t ,*constpcName, usStackDepth,  *constpvParameters, UBaseType_t uxPriority, TaskHandle_t *constpvCreatedTask, xCoreID)

    #[cfg(feature = "logging-encoders-raw")]
    unsafe {
        xTaskCreatePinnedToCore(
            Some(tasks::control_spi_encoders),
            CString::new("control_spi_encoders").unwrap().as_ptr(),
            8192,
            std::ptr::null_mut(),
            5,
            std::ptr::null_mut(),
            0,
        );
    }

    unsafe {
        xTaskCreatePinnedToCore(
            Some(tasks::reloj_debug),
            CString::new("reloj_debug").unwrap().as_ptr(),
            4096,
            std::ptr::null_mut(),
            5,
            std::ptr::null_mut(),
            1,
        );
    }

    loop {
        FreeRtos::delay_ms(30_000);
        println!("[main] alive");
    }
}

mod perifericos;
mod tasks;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::sys::xTaskCreatePinnedToCore;
use std::ffi::CString;fn main() -> anyhow::Result<()> {
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
            Some(tasks::task1),
            CString::new("Task 1").unwrap().as_ptr(),
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
    // unsafe {
    //     xTaskCreatePinnedToCore(
    //         Some(tasks::task2),
    //         CString::new("Task 2").unwrap().as_ptr(),
    //         4096,
    //         std::ptr::null_mut(),
    //         5,
    //         std::ptr::null_mut(),
    //         1,
    //     );
    // }
    
    //    unsafe {
    //     xTaskCreatePinnedToCore(
    //         Some(tasks::task3),
    //         CString::new("Task 2").unwrap().as_ptr(),
    //         4096,
    //         std::ptr::null_mut(),
    //         5,
    //         std::ptr::null_mut(),
    //         0,
    //     );
    // }
    
    // loop {
    //     println!("Hello From Main");
    //     FreeRtos::delay_ms(500);
    // }
}




// use esp_idf_svc::hal::delay::FreeRtos;
// use esp_idf_svc::sys::xTaskCreatePinnedToCore;
// use std::ffi::CString;

// fn main() -> anyhow::Result<()> {
//     esp_idf_svc::sys::link_patches();
//     // Inicializa el logger de esp-idf para que `println!` y logs de Rust se vean en UART
//     esp_idf_svc::log::EspLogger::initialize_default();

//     unsafe {
//         xTaskCreatePinnedToCore(
//             Some(tasks::lectoras),
//             CString::new("lectoras").unwrap().as_ptr(),
//             8192,
//             std::ptr::null_mut(),
//             5,
//             std::ptr::null_mut(),
//             0,
//         );
//     }

//     loop {
//         FreeRtos::delay_ms(30_000);
//         println!("[main] alive");
//     }
// }
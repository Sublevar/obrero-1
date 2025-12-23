use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{PinDriver, Pull, Gpio34, Gpio35};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::sys::{xTaskCreatePinnedToCore,esp_timer_get_time};
use std::ffi::CString;

unsafe extern "C" fn task1(_: *mut core::ffi::c_void) {
    loop {
        println!("Task 1 Entered");
        FreeRtos::delay_ms(1000);
    }
}

unsafe extern "C" fn task2(_: *mut core::ffi::c_void) {
    loop {
        println!("Task 2 Entered");
        FreeRtos::delay_ms(2000);
    }
}
unsafe extern "C" fn task3(_: *mut core::ffi::c_void) {
       let start_time = esp_timer_get_time();
    
    loop {
        let current_time_us = esp_timer_get_time();
        let elapsed_us = current_time_us - start_time;
        
        // Convertir a diferentes unidades
        let total_ms = elapsed_us / 1000;
        let total_seconds = elapsed_us / 1_000_000;
        let total_minutes = total_seconds / 60;
        let total_hours = total_minutes / 60;
        
        // Calcular componentes de tiempo
        let hours = total_hours;
        let minutes = total_minutes % 60;
        let seconds = total_seconds % 60;
        let milliseconds = total_ms % 1000;
        
        println!("Task 3 - Current Time:");
        println!("  Microsegundos totales: {}", elapsed_us);
        println!("  Milisegundos totales: {}", total_ms);
        println!("  Tiempo formateado: {:02}:{:02}:{:02}.{:03}", 
                 hours, minutes, seconds, milliseconds);
   
        //FreeRtos::delay_ms(100);

    }
}fn main() -> anyhow::Result<()> {
    //codigo que partchea RUST para operar en ESP32
    esp_idf_svc::sys::link_patches();

    //https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/freertos_idf.html#tasks
    //xTaskCreatePinnedToCore(TaskFunction_t ,*constpcName, usStackDepth,  *constpvParameters, UBaseType_t uxPriority, TaskHandle_t *constpvCreatedTask, xCoreID)

    unsafe {
        xTaskCreatePinnedToCore(
            Some(task1),
            CString::new("Task 1").unwrap().as_ptr(),
            4096,
            std::ptr::null_mut(),
            5,
            std::ptr::null_mut(),
            1,
        );
    }
    unsafe {
        xTaskCreatePinnedToCore(
            Some(task2),
            CString::new("Task 2").unwrap().as_ptr(),
            4096,
            std::ptr::null_mut(),
            5,
            std::ptr::null_mut(),
            1,
        );
    }
    
       unsafe {
        xTaskCreatePinnedToCore(
            Some(task3),
            CString::new("Task 2").unwrap().as_ptr(),
            4096,
            std::ptr::null_mut(),
            5,
            std::ptr::null_mut(),
            0,
        );
    }
    
    loop {
        println!("Hello From Main");
        FreeRtos::delay_ms(500);
    }
}
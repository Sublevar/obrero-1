use esp_idf_svc::sys::esp_timer_get_time;

#[allow(dead_code)]
pub unsafe extern "C" fn task3(_: *mut core::ffi::c_void) {
    let start_time = esp_timer_get_time();

    loop {
        let current_time_us = esp_timer_get_time();
        let elapsed_us = current_time_us - start_time;

        let total_ms = elapsed_us / 1000;
        let total_seconds = elapsed_us / 1_000_000;
        let total_minutes = total_seconds / 60;
        let total_hours = total_minutes / 60;

        let hours = total_hours;
        let minutes = total_minutes % 60;
        let seconds = total_seconds % 60;
        let milliseconds = total_ms % 1000;

        println!("Task 3 - Current Time:");
        println!("  Microsegundos totales: {}", elapsed_us);
        println!("  Milisegundos totales: {}", total_ms);
        println!(
            "  Tiempo formateado: {:02}:{:02}:{:02}.{:03}",
            hours, minutes, seconds, milliseconds
        );

        //FreeRtos::delay_ms(100);
    }
}

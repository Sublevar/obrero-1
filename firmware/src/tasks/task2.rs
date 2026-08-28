use esp_idf_svc::hal::delay::FreeRtos;

#[allow(dead_code)]
pub unsafe extern "C" fn task2(_: *mut core::ffi::c_void) {
    loop {
        eprintln!("Task 2 Entered");
        FreeRtos::delay_ms(2000);
    }
}

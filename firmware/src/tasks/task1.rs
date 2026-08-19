use esp_idf_svc::hal::delay::FreeRtos;

pub unsafe extern "C" fn task1(_: *mut core::ffi::c_void) {
    loop {
        println!("Task 1 Entered");
        FreeRtos::delay_ms(10000);
    }
}

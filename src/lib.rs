#[no_mangle]
extern "C" fn do_cpu_loop() -> () {
    let mut counter = 0u64;
    loop {
        counter += 1;
        if counter >= 9_000_000_000 {
            counter = 0;
        }
    }
}

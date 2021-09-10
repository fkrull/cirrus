extern "C" {
    fn ResticMain();
}

pub fn restic_main() -> ! {
    unsafe {
        ResticMain();
    }
    std::process::exit(0)
}

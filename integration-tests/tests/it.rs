use integration_tests::Workdir;

mod cirrus_core;

pub fn new_workdir() -> Workdir {
    Workdir::new(env!("CARGO_PKG_NAME")).unwrap()
}

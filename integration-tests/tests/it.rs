use integration_tests::Workdir;

mod cirrus;
mod cirrus_core;

pub fn new_workdir() -> Workdir {
    Workdir::new("test-binary")
}

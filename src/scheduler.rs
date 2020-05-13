use crate::App;
use std::time::Duration;
use std::{sync::Arc, thread, thread::JoinHandle};

pub fn start_scheduler(app: Arc<App>) -> anyhow::Result<JoinHandle<()>> {
    let handle = thread::Builder::new()
        .name("scheduler-thread".to_string())
        .spawn(move || scheduler(app))?;
    Ok(handle)
}

fn scheduler(_app: Arc<App>) {
    loop {
        println!("(not yet) scheduling...");
        thread::sleep(Duration::from_secs(10));
    }
}

use crate::App;
use log::info;
use std::{sync::Arc, thread, thread::JoinHandle, time::Duration};

pub fn start_scheduler(app: Arc<App>) -> anyhow::Result<JoinHandle<()>> {
    let handle = thread::Builder::new()
        .name("scheduler-thread".to_string())
        .spawn(move || scheduler(app))?;
    Ok(handle)
}

fn scheduler(_app: Arc<App>) {
    loop {
        info!("(not yet) scheduling...");
        thread::sleep(Duration::from_secs(10));
    }
}

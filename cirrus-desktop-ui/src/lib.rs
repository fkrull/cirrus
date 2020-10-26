use cirrus_daemon::job;
use log::info;
use std::sync::Arc;

mod notifications;

fn run_status_icon() -> eyre::Result<()> {
    let mut app = systray::Application::new()?;
    app.set_tooltip("Cirrus Backup")?;
    app.add_menu_item("Test", |_| -> std::io::Result<()> {
        info!("test menu item");
        Ok(())
    })?;
    app.add_menu_separator()?;
    app.add_menu_item("Exit", |_| -> std::io::Result<()> {
        std::process::exit(0);
    })?;
    loop {
        app.wait_for_message()?;
    }
}

#[derive(Debug)]
pub struct DesktopUi {
    appconfig: Arc<cirrus_core::appconfig::AppConfig>,
    notifications: notifications::Notifications,
}

impl DesktopUi {
    pub fn new(appconfig: Arc<cirrus_core::appconfig::AppConfig>) -> eyre::Result<Self> {
        Ok(Self {
            appconfig,
            notifications: notifications::Notifications::new()?,
        })
    }
}

#[async_trait::async_trait]
impl cirrus_actor::Actor for DesktopUi {
    type Message = job::StatusChange;
    type Error = eyre::Report;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        match message.new_status {
            job::Status::Started => {
                if self.appconfig.daemon.desktop.notifications.started {
                    self.notifications.notify_job_started(&message.job)?;
                }
            }
            job::Status::FinishedSuccessfully => {
                if self.appconfig.daemon.desktop.notifications.success {
                    self.notifications.notify_job_succeeded(&message.job)?;
                }
            }
            job::Status::FinishedWithError => {
                if self.appconfig.daemon.desktop.notifications.failure {
                    self.notifications.notify_job_failed(&message.job)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn on_start(&mut self) -> Result<(), Self::Error> {
        std::thread::spawn(|| {
            run_status_icon().unwrap();
        });
        Ok(())
    }
}

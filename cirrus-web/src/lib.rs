use crate::assets::{static_files::StaticFiles, templates::Template};
use cirrus_daemon::Daemon;
use rocket::routes;

mod assets;
mod routes;

pub async fn launch(daemon: Daemon) -> anyhow::Result<()> {
    rocket::ignite()
        .manage(daemon)
        .mount("/", routes![routes::index])
        .mount("/static", StaticFiles)
        .attach(Template::fairing())
        .launch()
        .await?;
    Ok(())
}

use crate::assets::templates::TemplateResult;
use cirrus_daemon::Daemon;
use rocket::{get, State};

#[get("/backups/<name>")]
pub async fn backup(name: String, daemon: State<'_, Daemon>) -> TemplateResult {
    todo!()
}

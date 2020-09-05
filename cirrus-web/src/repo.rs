use crate::assets::templates::TemplateResult;
use cirrus_daemon::Daemon;
use rocket::{get, State};

#[get("/repos/<name>")]
pub async fn repo(name: String, daemon: State<'_, Daemon>) -> TemplateResult {
    todo!()
}

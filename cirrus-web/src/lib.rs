use crate::assets::templates::{NoContext, Template};
use rocket::{get, routes};

mod assets;
mod static_files;

#[get("/")]
fn index() -> Template {
    Template::render("index.html", NoContext {})
}

pub async fn launch() -> anyhow::Result<()> {
    rocket::ignite()
        .mount("/", routes![index])
        .mount("/static", static_files::StaticFiles)
        .attach(Template::fairing())
        .launch()
        .await?;
    Ok(())
}

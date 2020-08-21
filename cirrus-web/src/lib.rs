use crate::assets::{
    static_files::StaticFiles,
    templates::{NoContext, Template},
};
use rocket::{get, routes};

mod assets;

#[get("/")]
fn index() -> Template {
    Template::render("index.html", NoContext {})
}

pub async fn launch() -> anyhow::Result<()> {
    rocket::ignite()
        .mount("/", routes![index])
        .mount("/static", StaticFiles)
        .attach(Template::fairing())
        .launch()
        .await?;
    Ok(())
}

use rocket::{get, routes};

mod static_files;

#[get("/")]
fn index() -> &'static str {
    "Hello Index"
}

pub async fn launch() -> anyhow::Result<()> {
    rocket::ignite()
        .mount("/", routes![index])
        .mount("/static", static_files::StaticFiles)
        .launch()
        .await?;
    Ok(())
}

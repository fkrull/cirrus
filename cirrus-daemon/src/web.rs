use rocket::{get, routes};

#[get("/")]
fn index() -> &'static str {
    "Hello Index"
}

pub async fn launch() -> anyhow::Result<()> {
    rocket::ignite().mount("/", routes![index]).launch().await?;
    Ok(())
}

use crate::assets::templates::{NoContext, Template};
use rocket::get;

#[get("/")]
pub(crate) fn index() -> Template {
    Template::render("index.html", NoContext {})
}

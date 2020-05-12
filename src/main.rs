#![feature(proc_macro_hygiene, decl_macro)]
use rocket::{get, routes};

mod config;

#[get("/")]
fn index() -> &'static str {
    "hello world"
}

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}

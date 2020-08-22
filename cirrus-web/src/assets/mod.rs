use rocket::http::ContentType;
use std::path::Path;

pub(crate) mod static_files;
pub(crate) mod templates;

fn content_type(path: impl AsRef<Path>) -> ContentType {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| ContentType::from_extension(ext))
        .unwrap_or_default()
}

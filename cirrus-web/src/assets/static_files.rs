use rocket::{
    handler::{Handler, Outcome},
    http::Method,
    Data, Request, Route,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct StaticFiles;

impl Into<Vec<Route>> for StaticFiles {
    fn into(self) -> Vec<Route> {
        vec![Route::new(Method::Get, "/<path..>", self)]
    }
}

#[rocket::async_trait]
impl Handler for StaticFiles {
    async fn handle<'r, 's: 'r>(&'s self, request: &'r Request<'_>, _data: Data) -> Outcome<'r> {
        let path = request
            .get_segments::<PathBuf>(0)
            .and_then(|result| result.ok())
            .unwrap_or_default();

        Outcome::from(request, assets_impl::get_file(path).await)
    }
}

#[cfg(not(feature = "bundled-assets"))]
mod assets_impl {
    use super::*;
    use rocket::response::NamedFile;

    pub(super) async fn get_file(path: impl AsRef<Path>) -> Option<NamedFile> {
        let mut abs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        abs_path.push("static");
        abs_path.push(path.as_ref());
        NamedFile::open(&abs_path).await.ok()
    }
}

#[cfg(feature = "bundled-assets")]
mod assets_impl {
    use super::*;
    use include_dir::{include_dir, Dir};
    use rocket::{http::ContentType, response::Responder};

    const FILES: Dir = include_dir!("static");

    pub(super) async fn get_file(path: impl AsRef<Path>) -> Option<BundledFile> {
        FILES.get_file(path).map(|file| {
            let bytes = file.contents();
            let content_type = file
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(|ext| ContentType::from_extension(ext))
                .unwrap_or_default();
            BundledFile {
                bytes,
                content_type,
            }
        })
    }

    #[derive(Responder)]
    pub(super) struct BundledFile {
        bytes: &'static [u8],
        content_type: ContentType,
    }
}

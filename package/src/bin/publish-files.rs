use xshell::*;

fn main() -> eyre::Result<()> {
    mkdir_p("public/downloads")?;

    let mut downloads_html = String::new();

    for file in read_dir("target")?.into_iter().filter(|p| p.is_file()) {
        cp(&file, "public/downloads/")?;
        let filename = file
            .file_name()
            .ok_or_else(|| eyre::eyre!("invalid filename {:?}", file))?
            .to_str()
            .ok_or_else(|| eyre::eyre!("non-utf8 filename {:?}", file))?;
        downloads_html.push_str(&format!(
            r#"<li><a href="/downloads/{}">{}</a></li>"#,
            filename, filename
        ));
    }

    for file in std::fs::read_dir("target")? {
        let file = file?;
        if file.path().is_dir() {
            break;
        }
    }

    let index = read_file("package/publish/index.html")?.replace("$DOWNLOADS", &downloads_html);
    write_file("public/index.html", index)?;

    Ok(())
}

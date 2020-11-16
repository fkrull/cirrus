use xshell::*;

fn main() -> eyre::Result<()> {
    mkdir_p("public")?;

    let mut downloads_html = String::new();
    for file in read_dir("public")? {
        let filename = file
            .file_name()
            .ok_or_else(|| eyre::eyre!("invalid filename {:?}", file))?
            .to_str()
            .ok_or_else(|| eyre::eyre!("non-utf8 filename {:?}", file))?;
        downloads_html.push_str(&format!(
            r#"<li><a href="/cirrus/{}">{}</a></li>"#,
            filename, filename
        ));
    }

    let index =
        read_file("build-scripts/publish/index.html")?.replace("$DOWNLOADS", &downloads_html);
    write_file("public/index.html", index)?;

    Ok(())
}

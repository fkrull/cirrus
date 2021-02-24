use crate::restic::{Event, Options, Restic};

pub async fn restic_version(restic: &Restic) -> eyre::Result<String> {
    let mut process = restic.run(
        None,
        &["version"],
        &Options {
            capture_output: true,
            ..Default::default()
        },
    )?;

    let mut output_line = None;
    loop {
        match process.next_event().await? {
            Event::StdoutLine(line) if !line.trim().is_empty() => {
                output_line = Some(line.trim().to_string());
            }
            Event::ProcessExit(_) => break,
            _ => {}
        }
    }

    output_line.ok_or_else(|| eyre::eyre!("couldn't get restic version"))
}

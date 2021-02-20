pub mod download;
pub mod urls;

pub fn restic_filename(target_os: &str) -> &str {
    match target_os {
        "windows" => "restic.exe",
        _ => "restic",
    }
}

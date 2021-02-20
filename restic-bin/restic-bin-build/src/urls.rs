const SHA256SUMS: &str = "
26c4c55363fc2a15122a97384a44c73fedf14b832721a0b4a86dc361468e7547  restic_0.12.0_aix_ppc64.bz2
c816973d0005248a7c6112026d9fa942e8e755748f60fd4a7b0b5ca4d578bd74  restic_0.12.0_darwin_amd64.bz2
9b5ac6a354462e1d547aa65f9c29632092a93861190b3c0a03534b1ec016a5e1  restic_0.12.0_freebsd_386.bz2
6410bf4446b371c8cc9dab16e0cdc1d0e5f21cfd3750a3a20f4c07c36befd5bc  restic_0.12.0_freebsd_amd64.bz2
832b7b0c67c63fcc6abb02d937a3b631f86a934cdf85879eb1a0da5705b05c65  restic_0.12.0_freebsd_arm.bz2
f2b2bb7385ee56d98659c4a0dbf42eca46227e10f92183a92934f4d96d523501  restic_0.12.0_linux_386.bz2
63d13d53834ea8aa4d461f0bfe32a89c70ec47e239b91f029ed10bd88b8f4b80  restic_0.12.0_linux_amd64.bz2
23c553049bbad7d777cd3b3d6065efa2edc2be13fd5eb1af15b43b6bfaf70bac  restic_0.12.0_linux_arm.bz2
e60e06956a8e8cdcba7688b6cb9b9815ada2b025e87b94d717172c02b9aa6c91  restic_0.12.0_linux_arm64.bz2
1eab0f66e1cf84017ad8aac6358d7bd50fef62477281b9492ccf772be20caf3c  restic_0.12.0_linux_mips.bz2
1fde906bc848a16734929e3d27c2223ab4e5be688b497cdcd8a0c4849931769b  restic_0.12.0_linux_mips64.bz2
ab8de228f748301d39294ae37b82aa068a47c9d36b42fd23c06afcb3375da1cd  restic_0.12.0_linux_mips64le.bz2
77310426d3e2e159f1ef2c8d498f17dc47cbeae310451377a2857f3ce9cd73c0  restic_0.12.0_linux_mipsle.bz2
e8c7827dae5c757ddfdd23ef8c97c24315a9c06dcecdde7ceb45dd21145d7a2a  restic_0.12.0_linux_ppc64le.bz2
8332935d27f531b6c85fe79f76625220391930506c5debb44895cd8269f58b07  restic_0.12.0_netbsd_386.bz2
969e56154298f0996396bf310bb745cfa549b2396765a49dc1611db1f118d2ca  restic_0.12.0_netbsd_amd64.bz2
53f3f97e369c874277a38fec36f2d533a865ad22c4ff8f06e4335f682c36b65a  restic_0.12.0_openbsd_386.bz2
0900453b3118e8907fd19a1bb4b56d29c3f09b20d1eaccc773e888f80761d065  restic_0.12.0_openbsd_amd64.bz2
97c9f305d684472b85157d1a2acc15364fa1999a25ddf50b40f5e76ef2fb8961  restic_0.12.0_solaris_amd64.bz2
a4239ce6da7f2934b3d732865bbfe7a866efbdcda80258bc4a247d3def967f9c  restic_0.12.0_windows_386.zip
0440615136eecfa56e9844e37679738622563c126c9cafb96433cec4ba11699a  restic_0.12.0_windows_amd64.zip
";

const BASE_URL: &str = "https://github.com/restic/restic/releases/download";

fn to_os(restic_os: &str) -> Option<&str> {
    match restic_os {
        "freebsd" => Some("freebsd"),
        "linux" => Some("linux"),
        "darwin" => Some("macos"),
        "netbsd" => Some("netbsd"),
        "openbsd" => Some("openbsd"),
        "windows" => Some("windows"),
        _ => None,
    }
}

fn to_arch(restic_arch: &str) -> Option<&str> {
    match restic_arch {
        "arm64" => Some("aarch64"),
        "arm" => Some("arm"),
        "mips" => Some("mips"),
        "mipsle" => Some("mips"),
        "ppc64le" => Some("powerpc64"),
        "386" => Some("x86"),
        "amd64" => Some("x86_64"),
        _ => None,
    }
}

fn to_endianness(restic_arch: &str) -> Option<&str> {
    match restic_arch {
        "mips" => Some("big"),
        "mipsle" => Some("little"),
        "ppc64le" => Some("little"),
        _ => None,
    }
}

#[derive(Debug)]
struct FileItem<'a> {
    checksum: &'a str,
    filename: &'a str,
    version: &'a str,
    os: &'a str,
    arch: &'a str,
    endianness: Option<&'a str>,
}

impl FileItem<'_> {
    fn parse(line: &str) -> Option<FileItem<'_>> {
        let mut line_parts = line.split_ascii_whitespace();
        let checksum = line_parts.next()?;
        let filename = line_parts.next()?;
        let mut name_parts = filename.rsplitn(2, '.').nth(1)?.split('_');
        let version = name_parts.nth(1)?;
        let os = name_parts.next().and_then(to_os)?;
        let restic_arch = name_parts.next()?;
        let arch = to_arch(restic_arch)?;
        let endianness = to_endianness(restic_arch);
        Some(FileItem {
            checksum,
            filename,
            version,
            os,
            arch,
            endianness,
        })
    }

    fn url(&self) -> String {
        format!("{}/v{}/{}", BASE_URL, self.version, self.filename)
    }
}

#[derive(Debug)]
pub struct UrlAndChecksum {
    pub url: String,
    pub checksum: String,
}

impl UrlAndChecksum {
    pub fn decompress_mode(&self) -> crate::download::DecompressMode {
        if self.url.ends_with(".zip") {
            crate::download::DecompressMode::UnzipSingle
        } else {
            crate::download::DecompressMode::Bunzip2
        }
    }
}

#[derive(Debug)]
pub struct Urls<'a> {
    items: Vec<FileItem<'a>>,
}

impl Default for Urls<'static> {
    fn default() -> Self {
        let items = SHA256SUMS
            .lines()
            .filter(|&s| !s.is_empty())
            .map(str::trim)
            .map(|o| {
                let item = FileItem::parse(o);
                if item.is_none() {
                    eprintln!("invalid line '{}'", o);
                }
                item
            })
            .filter_map(|o| o)
            .collect();
        Self { items }
    }
}

impl Urls<'_> {
    pub fn url_and_checksum(
        &self,
        target_os: &str,
        target_arch: &str,
        target_endian: &str,
    ) -> Option<UrlAndChecksum> {
        self.items
            .iter()
            .find(|&o| {
                o.os == target_os
                    && o.arch == target_arch
                    && (o.endianness.is_none() || o.endianness == Some(target_endian))
            })
            .map(|o| UrlAndChecksum {
                url: o.url(),
                checksum: o.checksum.to_owned(),
            })
    }
}

/*fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let items = SHA256SUMS
        .lines()
        .filter(|&s| !s.is_empty())
        .map(str::trim)
        .map(|o| {
            let item = FileItem::parse(o);
            if item.is_none() {
                eprintln!("invalid line '{}'", o);
            }
            item
        })
        .filter_map(|o| o);

    let urls_file = format!("{}/urls.rs", env!("CARGO_MANIFEST_DIR"));

    {
        let mut f = File::create(&urls_file)?;
        writeln!(f, "pub fn url(os: &str, arch: &str, endian: &str) -> Option<(&'static str, &'static str)> {{ match (os, arch, endian) {{")?;
        for item in items {
            writeln!(
                f,
                r#"("{}", "{}", {}) => Some(("{}", "{}")),"#,
                item.os,
                item.arch,
                item.endianness
                    .map(|s| format!(r#""{}""#, s))
                    .unwrap_or_else(|| "_".to_string()),
                item.url(),
                item.checksum
            )?;
        }
        writeln!(f, "_ => None }} }}")?;
    }

    std::process::Command::new("rustfmt")
        .arg(&urls_file)
        .spawn()?
        .wait()?;

    Ok(())
}
*/

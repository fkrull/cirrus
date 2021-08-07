use crate::TargetConfig;

const SHA256SUMS: &str = "
d3ebd06d4b88d5e4393e19b093fc74c773cd41db3d3a04662864934d5cf7dd05  restic_0.12.1_aix_ppc64.bz2
e41dc72ece30584c3e9c7772ba01a9f17e4e348805521382d16299e4694ac467  restic_0.12.1_darwin_amd64.bz2
575a6a7a4c23274aefb4eff8c0614036cc1999f108142741ce5296e4ce00811b  restic_0.12.1_darwin_arm64.bz2
a10a8b566860339bfd6832fc9073862c8689a1645236ad3d4eafa500f9c536a4  restic_0.12.1_freebsd_386.bz2
88f70507c3d00c6db0700498561444ba6ca5eff3afff4e0eecf96e7ac3668230  restic_0.12.1_freebsd_amd64.bz2
b1213c190d359872abf866bbfbd98b8140e16177157d241330b2ad172fa59daa  restic_0.12.1_freebsd_arm.bz2
a5581e05f792ca9ddec49004a9e3c9d203663e1b2ab330364d1e6ccb32bd8226  restic_0.12.1_linux_386.bz2
11d6ee35ec73058dae73d31d9cd17fe79661090abeb034ec6e13e3c69a4e7088  restic_0.12.1_linux_amd64.bz2
f27c3b271ad36896e22e411dea4c1c14d5ec75a232538c62099771ab7472765a  restic_0.12.1_linux_arm.bz2
c7e58365d0b888a60df772e7857ce8a0b53912bbd287582e865e3c5e17db723f  restic_0.12.1_linux_arm64.bz2
ba1320c819ee2b6e29fe38ea4df592813e7219a89175313556110775f2204201  restic_0.12.1_linux_mips.bz2
959bfdfe33740591330185406539399037eace2cd21bad62dc057db6ffd30656  restic_0.12.1_linux_mips64.bz2
e7c7c93448d7780b741496d34b10423f266ba09a8ebf1093b6d186e1f4c9e60a  restic_0.12.1_linux_mips64le.bz2
4f3e5adb0523a6811d21570838c9f061b7c9bb01264be518d0ed55039ac42547  restic_0.12.1_linux_mipsle.bz2
086848f2d4683ed2d581b584648d5c9c1bfe9ff61b85005c8a6477079f58b95d  restic_0.12.1_linux_ppc64le.bz2
bd6f57c36d0cf7393e1dcf6912c36887715864945fa06c457f135f9ea33fcf41  restic_0.12.1_linux_s390x.bz2
b396b58b9729c83406ade3cd3f6d52820a7ff6cf36cd4a59eb9d87ee267591fc  restic_0.12.1_netbsd_386.bz2
626ca456089857683c1ab8a5e3eda282837f7ed466ecf1a3c2cdd30e1b309c35  restic_0.12.1_netbsd_amd64.bz2
054cb9f42c4aca898ef078ddb7b138517c6f9f80225f9c7204f6ee00b9b93134  restic_0.12.1_openbsd_386.bz2
e7ae22a62f42e92811bb79ed2a268d4794a640a1d61282985f5dfd1b1d583b60  restic_0.12.1_openbsd_amd64.bz2
0bec24bf1d313b22de9c879bf3803256f945be419f23db4e58fdb73c3f15ec31  restic_0.12.1_solaris_amd64.bz2
8c1c0d5652d1d4a77c1c48526fa46eedbaf2d57b96b5a9e632c2b4917449a912  restic_0.12.1_windows_386.zip
f430a8069d7fac26e93994f8d89419e5285acbc0fb4514c89f427a070614af2e  restic_0.12.1_windows_amd64.zip
";

const BASE_URL: &str = "https://github.com/restic/restic/releases/download";

#[derive(Debug, thiserror::Error)]
#[error("invalid input line '{0}'")]
struct InvalidLine(String);

fn matches_os(os: &target_lexicon::OperatingSystem, restic_os: &str) -> bool {
    use target_lexicon::OperatingSystem;

    match os {
        OperatingSystem::Darwin | OperatingSystem::MacOSX { .. } => restic_os == "darwin",
        OperatingSystem::Freebsd => restic_os == "freebsd",
        OperatingSystem::Linux => restic_os == "linux",
        OperatingSystem::Netbsd => restic_os == "netbsd",
        OperatingSystem::Openbsd => restic_os == "openbsd",
        OperatingSystem::Solaris => restic_os == "solaris",
        OperatingSystem::Windows => restic_os == "windows",
        _ => false,
    }
}

fn matches_arch(arch: &target_lexicon::Architecture, restic_arch: &str) -> bool {
    use target_lexicon::{
        Aarch64Architecture, Architecture, Mips32Architecture, Mips64Architecture,
    };

    match arch {
        // TODO: should this be narrower rather than "all arm"?
        Architecture::Arm(_) => restic_arch == "arm",
        Architecture::Aarch64(Aarch64Architecture::Aarch64) => restic_arch == "arm64",
        Architecture::X86_32(_) => restic_arch == "386",
        Architecture::Mips32(Mips32Architecture::Mips) => restic_arch == "mips",
        Architecture::Mips32(Mips32Architecture::Mipsel) => restic_arch == "mipsle",
        Architecture::Mips64(Mips64Architecture::Mips64) => restic_arch == "mips64",
        Architecture::Mips64(Mips64Architecture::Mips64el) => restic_arch == "mips64le",
        Architecture::Powerpc64 => restic_arch == "ppc64",
        Architecture::Powerpc64le => restic_arch == "ppc64le",
        Architecture::X86_64 => restic_arch == "amd64",
        _ => false,
    }
}

#[derive(Debug)]
struct FileItem<'a> {
    checksum: &'a str,
    filename: &'a str,
    version: &'a str,
    os: &'a str,
    arch: &'a str,
}

impl FileItem<'_> {
    fn parse(line: &str) -> Result<FileItem<'_>, InvalidLine> {
        let (checksum, filename, version, os, arch) =
            Self::parts(line).ok_or_else(|| InvalidLine(line.to_string()))?;

        Ok(FileItem {
            checksum,
            filename,
            version,
            os,
            arch,
        })
    }

    fn parts(line: &str) -> Option<(&str, &str, &str, &str, &str)> {
        let mut line_parts = line.split_ascii_whitespace();
        let checksum = line_parts.next()?;
        let filename = line_parts.next()?;
        let mut name_parts = filename.rsplitn(2, '.').nth(1)?.split('_');
        let version = name_parts.nth(1)?;
        let os = name_parts.next()?;
        let arch = name_parts.next()?;
        Some((checksum, filename, version, os, arch))
    }

    fn url(&self) -> String {
        format!("{}/v{}/{}", BASE_URL, self.version, self.filename)
    }

    fn matches_target(&self, target: &TargetConfig) -> bool {
        matches_arch(&target.triple.architecture, self.arch)
            && matches_os(&target.triple.operating_system, self.os)
    }

    fn url_and_checksum(&self) -> UrlAndChecksum {
        UrlAndChecksum {
            url: self.url(),
            checksum: self.checksum.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct UrlAndChecksum {
    pub url: String,
    pub checksum: String,
}

impl UrlAndChecksum {
    pub fn decompress_mode(&self) -> crate::download_restic::downloader::DecompressMode {
        if self.url.ends_with(".zip") {
            crate::download_restic::downloader::DecompressMode::UnzipSingle
        } else {
            crate::download_restic::downloader::DecompressMode::Bunzip2
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
            // unwrap because we're parsing from embedded string
            .map(|o| FileItem::parse(o).unwrap())
            .collect();
        Self { items }
    }
}

impl Urls<'_> {
    pub fn url_and_checksum(&self, target: &TargetConfig) -> Option<UrlAndChecksum> {
        self.items
            .iter()
            .find(|o| o.matches_target(target))
            .map(|o| o.url_and_checksum())
    }
}

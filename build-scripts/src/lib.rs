pub mod container_image;
pub mod generate_icons;
pub mod get_version;
pub mod package;
pub mod update_vendor;

#[derive(Debug)]
pub struct TargetVars {
    pub go_os: &'static str,
    pub go_arch: &'static str,
    pub go_arm: Option<&'static str>,
    pub container_platform: &'static str,
    pub extension: &'static str,
}

impl TargetVars {
    pub fn for_target(target: &str) -> eyre::Result<TargetVars> {
        Ok(match target {
            "aarch64-unknown-linux-gnu" | "aarch64-unknown-linux-musl" => TargetVars {
                go_os: "linux",
                go_arch: "arm64",
                go_arm: None,
                container_platform: "linux/arm64/v8",
                extension: "",
            },
            "arm-unknown-linux-musleabihf" | "arm-unknown-linux-gnueabihf" => TargetVars {
                go_os: "linux",
                go_arch: "arm",
                go_arm: Some("6"),
                container_platform: "linux/arm32/v6",
                extension: "",
            },
            "armv7-unknown-linux-gnueabihf" | "armv7-unknown-linux-musleabihf" => TargetVars {
                go_os: "linux",
                go_arch: "arm",
                go_arm: Some("7"),
                container_platform: "linux/arm32/v7",
                extension: "",
            },
            "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => TargetVars {
                go_os: "linux",
                go_arch: "amd64",
                go_arm: None,
                container_platform: "linux/amd64",
                extension: "",
            },
            "x86_64-pc-windows-gnu" | "x86_64-pc-windows-msvc" => TargetVars {
                go_os: "windows",
                go_arch: "amd64",
                go_arm: None,
                container_platform: "windows/amd64",
                extension: ".exe",
            },
            _ => eyre::bail!("unknown target {}", target),
        })
    }
}

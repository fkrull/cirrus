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
    pub container_arch: &'static str,
    pub extension: &'static str,
    pub uses_dbus: bool,
}

impl TargetVars {
    pub fn for_target(target: &str) -> eyre::Result<TargetVars> {
        Ok(match target {
            "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => TargetVars {
                go_os: "linux",
                go_arch: "amd64",
                go_arm: None,
                container_arch: "amd64",
                extension: "",
                uses_dbus: true,
            },
            "armv7-unknown-linux-gnueabihf" => TargetVars {
                go_os: "linux",
                go_arch: "arm",
                go_arm: Some("7"),
                container_arch: "arm32v7",
                extension: "",
                uses_dbus: true,
            },
            "arm-unknown-linux-musl" => TargetVars {
                go_os: "linux",
                go_arch: "arm",
                go_arm: Some("6"),
                container_arch: "arm32v6",
                extension: "",
                uses_dbus: true,
            },
            "aarch64-unknown-linux-gnu" | "aarch64-unknown-linux-musl" => TargetVars {
                go_os: "linux",
                go_arch: "arm64",
                go_arm: None,
                container_arch: "arm64v8",
                extension: "",
                uses_dbus: true,
            },
            "x86_64-pc-windows-gnu" | "x86_64-pc-windows-msvc" => TargetVars {
                go_os: "windows",
                go_arch: "amd64",
                go_arm: None,
                container_arch: "amd64",
                extension: ".exe",
                uses_dbus: false,
            },
            _ => eyre::bail!("unknown target {}", target),
        })
    }
}

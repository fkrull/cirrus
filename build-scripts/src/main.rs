use build_scripts::*;

/// cirrus build scripts
#[derive(argh::FromArgs)]
struct Args {
    #[argh(subcommand)]
    cmd: Cmd,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
pub enum Cmd {
    ContainerImage(container_image::Args),
    GenerateIcons(generate_icons::Args),
    GetVersion(get_version::Args),
    Package(package::Args),
    UpdateVendor(update_vendor::Args),
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    match args.cmd {
        Cmd::ContainerImage(args) => container_image::main(args),
        Cmd::GenerateIcons(args) => generate_icons::main(args),
        Cmd::GetVersion(args) => get_version::main(args),
        Cmd::Package(args) => package::main(args),
        Cmd::UpdateVendor(args) => update_vendor::main(args),
    }
}

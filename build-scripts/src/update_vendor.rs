use xshell::*;

const RESTIC_GIT_URL: &str = "https://github.com/restic/restic.git";

/// Update vendored subtrees.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "update-vendor")]
pub struct Args {
    /// revision of the restic git to update to
    #[argh(option)]
    restic: Option<String>,
    /// create the subtree instead of updating it
    #[argh(switch)]
    add: bool,
}

pub fn main(args: Args) -> eyre::Result<()> {
    let sh = Shell::new()?;
    if let Some(rev) = args.restic {
        update(&sh, &rev, "vendor/restic", RESTIC_GIT_URL, args.add)?;
    }
    Ok(())
}

fn update(sh: &Shell, rev: &str, prefix: &str, url: &str, add: bool) -> eyre::Result<()> {
    let msg = format!("Update {prefix} to {rev}");
    let subtree_cmd = if add { "add" } else { "pull" };
    cmd!(
        sh,
        "git subtree {subtree_cmd} --squash --message {msg} --prefix {prefix} {url} {rev}"
    )
    .run()?;
    Ok(())
}

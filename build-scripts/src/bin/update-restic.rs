use xshell::*;

const RESTIC_GIT_URL: &'static str = "https://github.com/restic/restic.git";

/// Update vendored restic git.
#[derive(argh::FromArgs)]
struct Args {
    /// revision in the restic git to update to
    #[argh(option)]
    rev: String,

    /// create the subtree instead of updating it
    #[argh(switch)]
    add: bool,
}

fn main() -> eyre::Result<()> {
    let sh = Shell::new()?;
    let args: Args = argh::from_env();
    let rev = args.rev;
    let msg = format!("Update restic to {rev}");
    let git_cmd = if args.add { "add" } else { "pull" };
    cmd!(
        sh,
        "git subtree {git_cmd} --squash --message {msg} --prefix restic {RESTIC_GIT_URL} {rev}"
    )
    .run()?;
    Ok(())
}

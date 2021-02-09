use crate::cli;
use cirrus_core::{
    model::{repo, Config},
    secrets::{SecretValue, Secrets},
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn write_color(text: &str, fg_color: Color) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(fg_color)))?;
    let result = stdout.write_all(text.as_bytes());
    stdout.reset().ok();
    result
}

fn print_secret(
    secrets: &Secrets,
    repo_name: &repo::Name,
    secret_name: &str,
    secret: &repo::Secret,
    show_passwords: bool,
) -> eyre::Result<()> {
    print!("{}.{} [{}] = ", repo_name.0, secret_name, secret.label());
    match secrets.get_secret(secret) {
        Ok(value) => {
            let msg = if show_passwords {
                value.0.as_str()
            } else {
                "***"
            };
            write_color(msg, Color::Green)?
        }
        Err(_) => write_color("<UNSET>", Color::Red)?,
    };

    println!();
    Ok(())
}

pub fn set(secrets: &Secrets, config: &Config, args: cli::secret::Set) -> eyre::Result<()> {
    let repo_name = repo::Name(args.repository);
    let secret_name = args.secret.map(repo::SecretName);
    let repo = config.repository(&repo_name)?;

    let (secret, value) = match secret_name {
        None => {
            let prompt = format!("Password for repository '{}': ", repo_name.0);
            let value = SecretValue::new(rpassword::read_password_from_tty(Some(&prompt))?);
            (&repo.password, value)
        }
        Some(secret_name) => {
            let secret = repo
                .secrets
                .get(&secret_name)
                .ok_or_else(|| eyre::eyre!("no such secret '{}'", secret_name.0))?;
            let prompt = format!("Value for secret '{}.{}': ", repo_name.0, secret_name.0);
            let value = SecretValue::new(rpassword::read_password_from_tty(Some(&prompt))?);
            (secret, value)
        }
    };
    secrets.set_secret(secret, value)
}

pub fn list(secrets: &Secrets, config: &Config, args: cli::secret::List) -> eyre::Result<()> {
    for (repo_name, repo) in &config.repositories.0 {
        print_secret(
            secrets,
            repo_name,
            "<password>",
            &repo.password,
            args.show_passwords,
        )?;
        for (secret_name, secret) in &repo.secrets {
            print_secret(
                secrets,
                repo_name,
                &secret_name.0,
                secret,
                args.show_passwords,
            )?;
        }
    }

    Ok(())
}

const SYSTEMD_UNIT: &str = include_str!("../resources/cirrus.service");

fn template(template: &str, context: &[(&str, String)]) -> String {
    let mut templated = template.to_owned();
    for (key, value) in context {
        let pattern = format!("{{{}}}", key);
        templated = templated.replace(&pattern, value);
    }
    templated
}

pub fn systemd_unit() -> eyre::Result<()> {
    use eyre::WrapErr;

    let exe = std::env::current_exe()
        .wrap_err("failed to get path to cirrus executable")?
        .into_os_string()
        .into_string()
        .map_err(|_| eyre::eyre!("cirrus executable path contains non-UTF-8 characters"))?;
    let context = [("cirrus_binary", exe)];
    print!("{}", template(SYSTEMD_UNIT, &context));
    Ok(())
}

pub fn bash_completions() -> eyre::Result<()> {
    use crate::cli::Cli;
    use clap::IntoApp;

    let mut app = Cli::into_app();
    clap_generate::generate::<clap_generate::generators::Bash, _>(
        &mut app,
        "cirrus",
        &mut std::io::stdout(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_template_string() {
        let templated = template(
            "{a} + {b} = {result}",
            &[
                ("a", "4".to_owned()),
                ("b", "7".to_owned()),
                ("result", "11".to_owned()),
            ],
        );
        assert_eq!(&templated, "4 + 7 = 11");
    }
}

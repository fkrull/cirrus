use crate::new_workdir;
use cirrus_core::restic::{Event, Options, Restic};
use futures::prelude::*;

#[tokio::test]
async fn check_wait_should_return_error_if_process_exits_with_unsuccessful_status_code() {
    let workdir = new_workdir().with_exit_status(1).unwrap();
    let restic = Restic::new_with_path(workdir.test_binary());

    let result = restic
        .run(None, std::iter::empty::<&str>(), &Options::default())
        .unwrap()
        .check_wait()
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn should_capture_stdout_and_stderr() {
    let workdir = new_workdir()
        .with_stdout("stdout1\nstdout2\nstdout3")
        .unwrap()
        .with_stderr("stderr1\nstderr2\nstderr3\n")
        .unwrap();
    let restic = Restic::new_with_path(workdir.test_binary());

    let mut process = restic
        .run(
            None,
            std::iter::empty::<&str>(),
            &Options {
                capture_output: true,
                ..Default::default()
            },
        )
        .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    while let Some(event) = process.next().await {
        match event.unwrap() {
            Event::StdoutLine(line) => stdout.push(line),
            Event::StderrLine(line) => stderr.push(line),
        }
    }
    assert_eq!(&stdout, &["stdout1", "stdout2", "stdout3"]);
    assert_eq!(&stderr, &["stderr1", "stderr2", "stderr3"]);
}

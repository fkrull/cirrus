use crate::new_workdir;
use cirrus_core::restic::{Options, Output, Restic};
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn check_wait_should_return_error_if_process_exits_with_unsuccessful_status_code() {
    let workdir = new_workdir().with_exit_status(1);
    let restic = Restic::new_with_path(workdir.test_binary());

    let result = restic
        .run(None, &[] as &[&str], &Options::default())
        .unwrap()
        .check_wait()
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn should_capture_stdout_and_stderr() {
    let workdir = new_workdir()
        .with_stdout("stdout1\nstdout2\nstdout3")
        .with_stderr("stderr1\nstderr2\nstderr3\n");
    let restic = Restic::new_with_path(workdir.test_binary());

    let mut process = restic
        .run(
            None,
            &[] as &[&str],
            &Options {
                stdout: Output::Capture,
                stderr: Output::Capture,
                ..Default::default()
            },
        )
        .unwrap();

    let mut stdout = String::new();
    let mut stderr = String::new();
    process
        .stdout()
        .as_mut()
        .unwrap()
        .read_to_string(&mut stdout)
        .await
        .unwrap();
    process
        .stderr()
        .as_mut()
        .unwrap()
        .read_to_string(&mut stderr)
        .await
        .unwrap();
    assert_eq!(&stdout, "stdout1\nstdout2\nstdout3");
    assert_eq!(&stderr, "stderr1\nstderr2\nstderr3\n");
}

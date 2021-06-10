use crate::Workdir;
use cirrus_core::restic::{Options, Restic};

#[tokio::test]
async fn wait_should_return_error_if_process_exits_with_unsuccessful_status_code() {
    let workdir = Workdir::new().unwrap().with_exit_status(1).unwrap();
    let restic = Restic::new(Some(workdir.bin().to_owned()));

    let result = restic
        .run(None, std::iter::empty::<&str>(), &Options::default())
        .unwrap()
        .wait()
        .await;

    assert!(result.is_err());
}

use cirrus_core::config::repo;
use cirrus_index::{
    Database, File, FileSize, Gid, Mode, Owner, Parent, Snapshot, SnapshotId, TreeHash, Type, Uid,
    Version,
};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use time::macros::datetime;
use time::Duration;

fn insert_duplicate_versions(c: &mut Criterion) {
    const SNAPSHOTS_COUNT: u32 = 20;
    const FILES_COUNT: u32 = 100;
    let test_data = (0..SNAPSHOTS_COUNT)
        .map(|s_idx| {
            let snapshot = Snapshot {
                snapshot_id: SnapshotId(s_idx.to_string()),
                backup: None,
                parent: None,
                tree_hash: TreeHash(s_idx.to_string()),
                hostname: "testhost".to_string(),
                username: "testuser".to_string(),
                time: datetime!(2022-11-11 06:07:13 +0),
                tags: vec![],
            };
            let files_and_versions = (0..FILES_COUNT)
                .map(|f_idx| {
                    let file = File {
                        parent: Parent(None),
                        name: f_idx.to_string(),
                        r#type: Type::File,
                    };
                    let version = Version {
                        owner: Owner {
                            uid: Uid(1000),
                            gid: Gid(1000),
                        },
                        size: Some(FileSize(1234)),
                        mode: Mode(0o644),
                        mtime: datetime!(2022-07-06 05:04:03.210 +0),
                        ctime: datetime!(2022-01-02 03:04:05.678 -9),
                    };
                    (file, version)
                })
                .collect();
            (snapshot, files_and_versions)
        })
        .collect();

    bench_insert(
        c,
        &format!("{SNAPSHOTS_COUNT} snapshots, {FILES_COUNT} files, only duplicates"),
        test_data,
    );
}

fn insert_no_duplicate_versions(c: &mut Criterion) {
    const SNAPSHOTS_COUNT: u32 = 20;
    const FILES_COUNT: u32 = 100;
    let test_data = (0..SNAPSHOTS_COUNT)
        .map(|s_idx| {
            let snapshot = Snapshot {
                snapshot_id: SnapshotId(s_idx.to_string()),
                backup: None,
                parent: None,
                tree_hash: TreeHash(s_idx.to_string()),
                hostname: "testhost".to_string(),
                username: "testuser".to_string(),
                time: datetime!(2022-11-11 06:07:13 +0),
                tags: vec![],
            };
            let files_and_versions = (0..FILES_COUNT)
                .map(|f_idx| {
                    let file = File {
                        parent: Parent(None),
                        name: f_idx.to_string(),
                        r#type: Type::File,
                    };
                    let version = Version {
                        owner: Owner {
                            uid: Uid(f_idx),
                            gid: Gid(0),
                        },
                        size: Some(FileSize(f_idx as u64)),
                        mode: Mode(0o644),
                        mtime: datetime!(2022-07-06 05:04:03.210 +0),
                        ctime: datetime!(2022-01-02 03:04:05.678 -9),
                    };
                    (file, version)
                })
                .collect();
            (snapshot, files_and_versions)
        })
        .collect();

    bench_insert(
        c,
        &format!("{SNAPSHOTS_COUNT} snapshots, {FILES_COUNT} files, no duplicates"),
        test_data,
    );
}

fn get_files_many_results(c: &mut Criterion) {
    const SNAPSHOTS_COUNT: u32 = 3;
    const FILES_COUNT: u32 = 1000;
    let test_data = (0..SNAPSHOTS_COUNT)
        .map(|s_idx| {
            let snapshot = Snapshot {
                snapshot_id: SnapshotId(s_idx.to_string()),
                backup: None,
                parent: None,
                tree_hash: TreeHash(s_idx.to_string()),
                hostname: format!("host {s_idx}"),
                username: "testuser".to_string(),
                time: datetime!(2022-01-01 00:00:00 +0)
                    .replace_second(s_idx as u8)
                    .unwrap(),
                tags: vec![],
            };
            let files_and_versions = (0..FILES_COUNT)
                .map(|f_idx| {
                    let file = File {
                        parent: Parent(Some("/tmp".to_string())),
                        name: f_idx.to_string(),
                        r#type: Type::File,
                    };
                    let version = Version {
                        owner: Owner {
                            uid: Uid(f_idx),
                            gid: Gid(f_idx),
                        },
                        size: Some(FileSize(12)),
                        mode: Mode(0o644),
                        mtime: datetime!(2022-07-06 05:04:03.210 +0),
                        ctime: datetime!(2022-01-02 03:04:05.678 -9),
                    };
                    (file, version)
                })
                .collect();
            (snapshot, files_and_versions)
        })
        .collect();

    bench_get_files(
        c,
        &format!("{SNAPSHOTS_COUNT} snapshots, {FILES_COUNT} files, many results"),
        test_data,
        Parent(Some("/tmp".to_string())),
        10000,
        1000,
    );
}

fn get_files_few_results(c: &mut Criterion) {
    const SNAPSHOTS_COUNT: u32 = 3;
    const FILES_COUNT: u32 = 1000;
    let test_data = (0..SNAPSHOTS_COUNT)
        .map(|s_idx| {
            let snapshot = Snapshot {
                snapshot_id: SnapshotId(s_idx.to_string()),
                backup: None,
                parent: None,
                tree_hash: TreeHash(s_idx.to_string()),
                hostname: format!("host {s_idx}"),
                username: "testuser".to_string(),
                time: datetime!(2022-01-01 00:00:00 +0)
                    .replace_second(s_idx as u8)
                    .unwrap(),
                tags: vec![],
            };
            let files_and_versions = (0..FILES_COUNT)
                .map(|f_idx| {
                    let file = File {
                        parent: Parent(Some((f_idx % 100).to_string())),
                        name: f_idx.to_string(),
                        r#type: Type::Dir,
                    };
                    let version = Version {
                        owner: Owner {
                            uid: Uid(f_idx),
                            gid: Gid(f_idx),
                        },
                        size: Some(FileSize(12)),
                        mode: Mode(0o644),
                        mtime: datetime!(2022-07-06 05:04:03.210 +0),
                        ctime: datetime!(2022-01-02 03:04:05.678 -9),
                    };
                    (file, version)
                })
                .collect();
            (snapshot, files_and_versions)
        })
        .collect();

    bench_get_files(
        c,
        &format!("{SNAPSHOTS_COUNT} snapshots, {FILES_COUNT} files, few results"),
        test_data,
        Parent(Some("50".to_string())),
        100,
        10,
    );
}

fn get_files_many_versions_few_results(c: &mut Criterion) {
    const SNAPSHOTS_COUNT: u32 = 200;
    const FILES_COUNT: u32 = 20;
    let test_data = (0..SNAPSHOTS_COUNT)
        .map(|s_idx| {
            let snapshot = Snapshot {
                snapshot_id: SnapshotId(s_idx.to_string()),
                backup: None,
                parent: None,
                tree_hash: TreeHash(s_idx.to_string()),
                hostname: format!("host {s_idx}"),
                username: "testuser".to_string(),
                time: datetime!(2022-01-01 00:00:00 +0) + Duration::seconds(s_idx as i64),
                tags: vec![],
            };
            let files_and_versions = (0..FILES_COUNT)
                .map(|f_idx| {
                    let file = File {
                        parent: Parent(None),
                        name: f_idx.to_string(),
                        r#type: Type::File,
                    };
                    let version = Version {
                        owner: Owner {
                            uid: Uid(0),
                            gid: Gid(0),
                        },
                        size: Some(FileSize(12)),
                        mode: Mode(0o644),
                        mtime: datetime!(2022-07-06 05:04:03.210 +0),
                        ctime: datetime!(2022-01-02 03:04:05.678 -9),
                    };
                    (file, version)
                })
                .collect();
            (snapshot, files_and_versions)
        })
        .collect();

    bench_get_files(
        c,
        &format!("{SNAPSHOTS_COUNT} snapshots, {FILES_COUNT} files"),
        test_data,
        Parent(None),
        10000,
        20,
    );
}

fn bench_insert(c: &mut Criterion, id: &str, test_data: Vec<(Snapshot, Vec<(File, Version)>)>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmp = tempfile::tempdir().unwrap();

    c.bench_with_input(
        BenchmarkId::new("insert", id),
        &test_data,
        |b, test_data| {
            let mut i = 0;
            b.iter_batched(
                || {
                    i += 1;
                    let db = rt
                        .block_on(Database::new(tmp.path(), &repo::Name(i.to_string())))
                        .unwrap();
                    (db, test_data.clone())
                },
                |(mut db, test_data)| {
                    for (snapshot, files_and_versions) in test_data {
                        let files = futures::stream::iter(files_and_versions.into_iter().map(Ok));
                        rt.block_on(db.import_files(&snapshot, files)).unwrap();
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );
}

fn bench_get_files(
    c: &mut Criterion,
    id: &str,
    test_data: Vec<(Snapshot, Vec<(File, Version)>)>,
    parent: Parent,
    limit: u64,
    expected: usize,
) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmp = tempfile::tempdir().unwrap();

    c.bench_with_input(
        BenchmarkId::new("get_files", id),
        &test_data,
        |b, test_data| {
            let mut i = 0;
            b.iter_batched(
                || {
                    i += 1;
                    let mut db = rt
                        .block_on(Database::new(tmp.path(), &repo::Name(i.to_string())))
                        .unwrap();
                    let snapshots = test_data.iter().map(|o| o.0.clone());
                    rt.block_on(db.import_snapshots(snapshots)).unwrap();
                    for (snapshot, files_and_versions) in test_data {
                        let files =
                            futures::stream::iter(files_and_versions.iter().cloned().map(Ok));
                        rt.block_on(db.import_files(snapshot, files)).unwrap();
                    }
                    db
                },
                |mut db| {
                    let ret = rt.block_on(db.get_files(&parent, limit)).unwrap().len();
                    assert_eq!(ret, expected);
                },
                BatchSize::SmallInput,
            );
        },
    );
}

criterion_group!(
    db,
    insert_duplicate_versions,
    insert_no_duplicate_versions,
    get_files_many_results,
    get_files_few_results,
    get_files_many_versions_few_results,
);
criterion_main!(db);

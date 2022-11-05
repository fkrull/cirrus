use cirrus_core::config::repo;
use cirrus_index::{
    Database, File, FileSize, Gid, Mode, Owner, Parent, Snapshot, SnapshotId, TreeHash, Type, Uid,
    Version,
};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use time::macros::datetime;

const SNAPSHOTS_COUNT: u32 = 20;
const FILES_COUNT: u32 = 100;

fn index_insert_duplicate_versions(c: &mut Criterion) {
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

    bench_index_insert(
        c,
        &format!("{SNAPSHOTS_COUNT} snapshots, {FILES_COUNT} files, only duplicates"),
        test_data,
    );
}

fn index_insert_no_duplicate_versions(c: &mut Criterion) {
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

    bench_index_insert(
        c,
        &format!("{SNAPSHOTS_COUNT} snapshots, {FILES_COUNT} files, no duplicates"),
        test_data,
    );
}

fn bench_index_insert(
    c: &mut Criterion,
    id: &str,
    test_data: Vec<(Snapshot, Vec<(File, Version)>)>,
) {
    let bench_rt = tokio::runtime::Runtime::new().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path();

    c.bench_with_input(
        BenchmarkId::new("index_insert", id),
        &test_data,
        |b, test_data| {
            let mut i = 0;
            b.to_async(&bench_rt).iter_batched(
                || test_data.clone(),
                |test_data| {
                    i += 1;
                    let i_copy = i;
                    async move {
                        let mut db = Database::new(path, &repo::Name(i_copy.to_string()))
                            .await
                            .unwrap();
                        for (snapshot, files_and_versions) in test_data {
                            let files =
                                futures::stream::iter(files_and_versions.into_iter().map(Ok));
                            db.import_files(&snapshot, files).await.unwrap();
                        }
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );
}

criterion_group!(
    index_insert,
    index_insert_duplicate_versions,
    index_insert_no_duplicate_versions
);
criterion_main!(index_insert);

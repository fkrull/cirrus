use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LsEntry {
    name: String,
    #[serde(rename = "type")]
    r#type: Type,
    path: String,
    uid: u32,
    gid: u32,
    mode: u32,
    permissions: String,
    #[serde(with = "time::serde::iso8601")]
    mtime: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    atime: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    ctime: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Type {
    Dir,
    File,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn should_parse_dir_item() {
        // language=JSON
        let json = r#"{
          "name": "a-directory",
          "type": "dir",
          "path": "/var/tmp/subdir/a-directory",
          "uid": 1000,
          "gid": 1000,
          "mode": 2147484157,
          "permissions": "drwxrwxr-x",
          "mtime": "2022-06-05T13:46:04.582083272+02:00",
          "atime": "2022-06-05T13:56:04.582083272+02:00",
          "ctime": "2022-06-05T13:16:04.582083272+02:00",
          "struct_type": "node"
        }"#;

        let item: LsEntry = serde_json::from_str(json).unwrap();

        assert_eq!(
            item,
            LsEntry {
                name: "a-directory".to_string(),
                r#type: Type::Dir,
                path: "/var/tmp/subdir/a-directory".to_string(),
                uid: 1000,
                gid: 1000,
                mode: 0o20000000775,
                permissions: "drwxrwxr-x".to_string(),
                mtime: datetime!(2022-06-05 13:46:04.582083272 +02:00),
                atime: datetime!(2022-06-05 13:56:04.582083272 +02:00),
                ctime: datetime!(2022-06-05 13:16:04.582083272 +02:00),
            }
        )
    }

    #[test]
    fn should_parse_file_item() {
        // language=JSON
        let json = r#"{
          "name": "test.yml",
          "type": "file",
          "path": "/test.yml",
          "uid": 0,
          "gid": 0,
          "mode": 384,
          "permissions": "-rw-------",
          "mtime": "2022-10-22T13:46:04.582083272+02:00",
          "atime": "2022-10-22T13:56:04.582083272+02:00",
          "ctime": "2022-10-22T13:16:04.582083272+02:00",
          "struct_type": "node"
        }"#;

        let item: LsEntry = serde_json::from_str(json).unwrap();

        assert_eq!(
            item,
            LsEntry {
                name: "test.yml".to_string(),
                r#type: Type::File,
                path: "/test.yml".to_string(),
                uid: 0,
                gid: 0,
                mode: 0o600,
                permissions: "-rw-------".to_string(),
                mtime: datetime!(2022-10-22 13:46:04.582083272 +02:00),
                atime: datetime!(2022-10-22 13:56:04.582083272 +02:00),
                ctime: datetime!(2022-10-22 13:16:04.582083272 +02:00),
            }
        )
    }
}

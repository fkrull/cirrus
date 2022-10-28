use crate::config::backup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(pub String);

impl Tag {
    pub fn for_backup(name: &backup::Name) -> Tag {
        Tag(format!("cirrus.{}", name.0))
    }

    pub fn backup_name(&self) -> Option<backup::Name> {
        self.0
            .strip_prefix("cirrus.")
            .or_else(|| self.0.strip_prefix("cirrus-backup-"))
            .map(|s| s.to_string())
            .map(backup::Name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_format_tag() {
        let name = backup::Name("my-cool-backup".to_string());

        let tag = Tag::for_backup(&name);

        assert_eq!(tag, Tag("cirrus.my-cool-backup".to_string()));
    }

    #[test]
    fn should_parse_old_style_tag() {
        let result = Tag("cirrus-backup-old-style-id".to_string()).backup_name();

        assert_eq!(result.unwrap(), backup::Name("old-style-id".to_string()));
    }

    #[test]
    fn should_parse_new_style_tag() {
        let result = Tag("cirrus.my-cool-backup".to_string()).backup_name();

        assert_eq!(result.unwrap(), backup::Name("my-cool-backup".to_string()));
    }

    #[test]
    fn should_parse_new_style_tag_with_weird_characters() {
        let result = Tag("cirrus.my cool backup.com".to_string()).backup_name();

        assert_eq!(
            result.unwrap(),
            backup::Name("my cool backup.com".to_string())
        );
    }

    #[test]
    fn should_not_parse_empty_string() {
        let result = Tag("".to_string()).backup_name();

        assert_eq!(result, None);
    }

    #[test]
    fn should_not_parse_tag_without_prefix() {
        let result = Tag("backup-test".to_string()).backup_name();

        assert_eq!(result, None);
    }

    #[test]
    fn should_not_parse_incorrect_prefix() {
        let result = Tag("cirrus-bkp-test".to_string()).backup_name();

        assert_eq!(result, None);
    }

    #[test]
    fn should_not_parse_incorrect_new_style_prefix() {
        let result = Tag("cirrus-backup.abc".to_string()).backup_name();

        assert_eq!(result, None);
    }
}

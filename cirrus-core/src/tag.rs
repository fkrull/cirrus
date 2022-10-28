use crate::config::backup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(pub String);

impl Tag {
    pub fn for_backup(name: &backup::Name) -> Tag {
        Tag(format!("cirrus.{}", name.0))
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
}

pub mod config {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    pub mod repo {
        use serde::{Deserialize, Serialize};
        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub enum Password {
            FromEnvVar(String),
        }

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub struct Name(pub String);

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub struct Url(pub String);

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub struct Definition {
            url: Url,
            password: Password,
        }
    }

    pub mod backup {
        use super::repo;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub struct Name(pub String);

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub struct Path(pub String);

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub struct Exclude(pub String);

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub struct Definition {
            repository: repo::Name,
            path: Path,
            excludes: Vec<Exclude>,
            extra_args: Vec<String>,
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
    pub struct Config {
        repositories: HashMap<repo::Name, repo::Definition>,
        backups: HashMap<backup::Name, backup::Definition>,
    }
}

fn main() {
    println!("Hello, world!");
}

use crate::model::backup;
use crate::model::repo;

pub trait Restic {
    fn init(&self, repo: &repo::Definition) -> anyhow::Result<()>;
    fn backup(&self, repo: &repo::Definition, backup: &backup::Definition) -> anyhow::Result<()>;
}

pub fn init(repo: &repo::Definition) -> anyhow::Result<()> {
    todo!()
}

pub fn backup(repo: &repo::Definition, backup: &backup::Definition) -> anyhow::Result<()> {
    todo!()
}

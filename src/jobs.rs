use crate::model::backup;
use chrono::{DateTime, Utc};
use std::sync::Mutex;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum JobStatus {
    Waiting,
    Running,
    FinishedWithError,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Job {
    pub name: backup::Name,
    pub definition: backup::Definition,
    pub status: JobStatus,
    pub last_start: Option<DateTime<Utc>>,
    pub last_finish: Option<DateTime<Utc>>,
}

impl Job {
    pub fn new(name: backup::Name, definition: backup::Definition) -> Self {
        Job {
            name,
            definition,
            status: JobStatus::Waiting,
            last_start: None,
            last_finish: None,
        }
    }

    pub fn running(&self) -> bool {
        self.status == JobStatus::Running
    }

    pub fn set_started(&mut self, start: DateTime<Utc>) {
        self.status = JobStatus::Running;
        self.last_start = Some(start);
    }

    pub fn set_finished_successful(&mut self, end: DateTime<Utc>) {
        self.status = JobStatus::Waiting;
        self.last_finish = Some(end);
    }

    pub fn set_finished_failed(&mut self, end: DateTime<Utc>) {
        self.status = JobStatus::FinishedWithError;
        self.last_finish = Some(end);
    }
}

#[derive(Debug, Default)]
pub struct JobsRepo {
    jobs: Mutex<Vec<Job>>,
}

impl JobsRepo {
    pub fn new(backups: impl IntoIterator<Item = (backup::Name, backup::Definition)>) -> Self {
        let jobs = backups
            .into_iter()
            .map(|(name, definition)| Job::new(name, definition))
            .collect();
        JobsRepo {
            jobs: Mutex::new(jobs),
        }
    }

    pub fn jobs(&self) -> impl Iterator<Item = Job> {
        self.jobs.lock().unwrap().clone().into_iter()
    }

    pub fn update(&self, jobs: impl Iterator<Item = Job>) {
        let mut vec = self.jobs.lock().unwrap();
        for job in jobs {
            if let Some(old) = vec.iter_mut().find(|old| old.name == job.name) {
                *old = job;
            }
        }
    }

    pub fn get(&self, name: &backup::Name) -> Option<Job> {
        self.jobs
            .lock()
            .unwrap()
            .iter()
            .find(|&job| &job.name == name)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::repo;
    use maplit::hashmap;
    use std::iter;

    #[test]
    fn should_get_and_update_jobs() -> anyhow::Result<()> {
        let name1 = backup::Name("1".to_string());
        let name2 = backup::Name("2".to_string());
        let definition1 = backup::Definition {
            repository: repo::Name("repo1".to_string()),
            ..Default::default()
        };
        let definition2 = backup::Definition {
            repository: repo::Name("repo2".to_string()),
            ..Default::default()
        };
        let timestamp = Utc::now();

        let repo = JobsRepo::new(hashmap! {
            name1.clone() => definition1.clone(),
            name2.clone() => definition2.clone(),
        });

        let mut jobs1 = repo.jobs().collect::<Vec<_>>();
        jobs1.sort_by_key(|job| job.name.clone());
        assert_eq!(
            jobs1,
            vec![
                Job::new(name1.clone(), definition1.clone()),
                Job::new(name2.clone(), definition2.clone()),
            ]
        );

        let mut job = jobs1.remove(1);
        job.set_started(timestamp);
        repo.update(iter::once(job));

        let mut jobs2 = repo.jobs().collect::<Vec<_>>();
        jobs2.sort_by_key(|job| job.name.clone());
        assert_eq!(
            jobs2,
            vec![
                Job::new(name1.clone(), definition1.clone()),
                Job {
                    status: JobStatus::Running,
                    last_start: Some(timestamp),
                    ..Job::new(name2.clone(), definition2.clone())
                },
            ]
        );

        Ok(())
    }

    #[test]
    fn should_get_job_by_name() -> anyhow::Result<()> {
        let backup1 = (backup::Name("1".to_string()), backup::Definition::default());
        let backup2 = (backup::Name("2".to_string()), backup::Definition::default());
        let repo = JobsRepo::new(vec![backup1.clone(), backup2.clone()]);

        let job = repo.get(&backup::Name("1".to_string())).unwrap();

        assert_eq!(job.name, backup1.0);
        assert_eq!(job.definition, backup1.1);
        Ok(())
    }

    #[test]
    fn should_not_get_missing_job_by_name() -> anyhow::Result<()> {
        let repo = JobsRepo::new(iter::empty());

        let result = repo.get(&backup::Name("test".to_string()));

        assert_eq!(result, None);
        Ok(())
    }
}
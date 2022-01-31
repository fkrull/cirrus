use serde::{Deserialize, Serialize};

pub mod cron;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Trigger {
    Cron(cron::Cron),
}

impl Trigger {
    pub fn next_schedule(&self, after: time::OffsetDateTime) -> eyre::Result<time::OffsetDateTime> {
        match self {
            Trigger::Cron(cron) => cron.next_schedule(after),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn should_deserialize_cron() {
        let json = json!({
            "cron": "2 * *"
        });
        let result = serde_json::from_value::<Trigger>(json);

        assert_eq!(
            result.unwrap(),
            Trigger::Cron(cron::Cron {
                cron: "2 * *".to_owned(),
                timezone: cron::Timezone::default()
            })
        );
    }

    #[test]
    fn should_deserialize_cron_with_timezone() {
        let json = json!({
            "cron": "blub",
            "timezone": "utc"
        });
        let result = serde_json::from_value::<Trigger>(json);

        assert_eq!(
            result.unwrap(),
            Trigger::Cron(cron::Cron {
                cron: "blub".to_owned(),
                timezone: cron::Timezone::Utc
            })
        );
    }
}

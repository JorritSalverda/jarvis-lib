use crate::model::*;
use serde::de::DeserializeOwned;
use serde_yaml;
use std::env;
use std::error::Error;
use std::fs;
use tracing::{debug, info};

pub trait SetDefaults {
    fn set_defaults(&mut self);
}

pub struct ConfigClientConfig {
    config_path: String,
}

impl ConfigClientConfig {
    pub fn new(config_path: String) -> Result<Self, Box<dyn Error>> {
        debug!("ConfigClientConfig::new(config_path: {})", config_path);
        Ok(Self { config_path })
    }

    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let config_path =
            env::var("CONFIG_PATH").unwrap_or_else(|_| "/configs/config.yaml".to_string());

        Self::new(config_path)
    }
}

pub struct ConfigClient {
    config: ConfigClientConfig,
}

impl ConfigClient {
    pub fn new(config: ConfigClientConfig) -> Self {
        Self { config }
    }

    pub fn read_config_from_file<T>(&self) -> Result<T, Box<dyn Error>>
    where
        T: DeserializeOwned + SetDefaults,
    {
        let config_file_contents = fs::read_to_string(&self.config.config_path)?;
        let mut config: T = serde_yaml::from_str(&config_file_contents)?;

        config.set_defaults();

        info!("Loaded config from {}", &self.config.config_path);

        Ok(config)
    }

    pub fn read_planner_config_from_file(&self) -> Result<SpotPricePlannerConfig, Box<dyn Error>> {
        let config_file_contents = fs::read_to_string(&self.config.config_path)?;
        let config: SpotPricePlannerConfig = serde_yaml::from_str(&config_file_contents)?;

        info!("Loaded planner config from {}", &self.config.config_path);

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntityType;
    use assert2::{check, let_assert};
    use chrono::naive::NaiveTime;
    use chrono::Weekday;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Config {
        pub location: String,
        pub entity_type: EntityType,
        pub entity_name: String,
    }

    impl SetDefaults for Config {
        fn set_defaults(&mut self) {}
    }

    #[test]
    fn read_config_from_file_returns_deserialized_test_file() {
        let_assert!(Ok(config) = ConfigClientConfig::new("test-config.yaml".to_string()));
        let config_client = ConfigClient::new(config);

        let_assert!(
            Ok(Config {
                location,
                entity_type,
                entity_name,
            }) = config_client.read_config_from_file()
        );

        check!(location == "My Home".to_string());
        check!(entity_type == EntityType::Device);
        check!(entity_name == "TP-Link HS110".to_string());
    }

    #[test]
    fn read_planner_config_from_file_returns_deserialized_test_file() {
        let_assert!(Ok(config) = ConfigClientConfig::new("test-config.yaml".to_string()));
        let config_client = ConfigClient::new(config);

        let_assert!(
            Ok(SpotPricePlannerConfig {
                plannable_local_time_slots,
                load_profile: LoadProfile { sections },
                ..
            }) = config_client.read_planner_config_from_file()
        );

        let_assert!([sec_0, sec_1, ..] = sections.as_slice());

        check!(sec_0.duration_seconds == 7200);
        check!(sec_0.power_draw_watt == 2000.0);
        check!(sec_1.duration_seconds == 1800);
        check!(sec_1.power_draw_watt == 8000.0);

        check!(plannable_local_time_slots.len() == 2);

        // Thursday time slots ...
        let_assert!(Some(thu_time_slots) = plannable_local_time_slots.get(&Weekday::Thu));
        let_assert!([slot_0, slot_1, ..] = thu_time_slots.as_slice());

        check!(slot_0.from == NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        check!(slot_0.till == NaiveTime::from_hms_opt(7, 0, 0).unwrap());
        check!(slot_1.from == NaiveTime::from_hms_opt(23, 0, 0).unwrap());
        check!(slot_1.till == NaiveTime::from_hms_opt(0, 0, 0).unwrap());

        // Satureday time slots ...
        let_assert!(Some(sat_time_slots) = plannable_local_time_slots.get(&Weekday::Sat));
        let_assert!([slot_0, ..] = sat_time_slots.as_slice());
        check!(slot_0.from == NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        check!(slot_0.till == NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }
}

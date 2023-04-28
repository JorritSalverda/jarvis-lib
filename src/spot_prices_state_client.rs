use crate::model::*;
use std::env;
use std::error::Error;
use std::fs;
use tracing::info;

pub struct SpotPricesStateClientConfig {
    state_file_path: String,
}

impl SpotPricesStateClientConfig {
    pub fn new(state_file_path: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            state_file_path: state_file_path.into(),
        })
    }

    pub async fn from_env() -> Result<Self, Box<dyn Error>> {
        let state_file_path =
            env::var("STATE_FILE_PATH").unwrap_or_else(|_| "/state/state.yaml".to_string());

        Self::new(&state_file_path)
    }
}

pub struct SpotPricesStateClient {
    config: SpotPricesStateClientConfig,
}

impl SpotPricesStateClient {
    pub fn new(config: SpotPricesStateClientConfig) -> SpotPricesStateClient {
        SpotPricesStateClient { config }
    }

    pub async fn from_env() -> Result<Self, Box<dyn Error>> {
        Ok(Self::new(SpotPricesStateClientConfig::from_env().await?))
    }

    pub fn read_state(&self) -> Result<Option<SpotPricesState>, Box<dyn std::error::Error>> {
        let state_file_contents = match fs::read_to_string(&self.config.state_file_path) {
            Ok(c) => c,
            Err(_) => return Ok(Option::None),
        };

        let last_state: Option<SpotPricesState> = match serde_yaml::from_str(&state_file_contents) {
            Ok(lm) => Some(lm),
            Err(_) => return Ok(Option::None),
        };

        info!("Read state file at {}", &self.config.state_file_path);

        Ok(last_state)
    }
}

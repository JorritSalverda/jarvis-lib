use std::error::Error;

use crate::config_client::{ConfigClient, SetDefaults};
use crate::planner_client::PlannerClient;
use crate::spot_prices_state_client::SpotPricesStateClient;
use serde::de::DeserializeOwned;

pub struct PlannerServiceConfig<T: ?Sized> {
    config_client: ConfigClient,
    spot_prices_state_client: SpotPricesStateClient,
    planner_client: Box<dyn PlannerClient<T>>,
}

impl<T> PlannerServiceConfig<T> {
    pub fn new(
        config_client: ConfigClient,
        spot_prices_state_client: SpotPricesStateClient,
        planner_client: Box<dyn PlannerClient<T>>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config_client,
            spot_prices_state_client,
            planner_client,
        })
    }
}

pub struct PlannerService<T> {
    config: PlannerServiceConfig<T>,
}

impl<T> PlannerService<T> {
    pub fn new(config: PlannerServiceConfig<T>) -> Self {
        Self { config }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + SetDefaults,
    {
        let config: T = self.config.config_client.read_config_from_file()?;

        let spot_prices_state = self.config.spot_prices_state_client.read_state()?;

        self.config.planner_client.plan(config, spot_prices_state)?;

        Ok(())
    }
}

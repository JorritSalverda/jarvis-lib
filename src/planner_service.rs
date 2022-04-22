use crate::config_client::{ConfigClient, SetDefaults};
use crate::model::*;
use crate::planner_client::PlannerClient;
use crate::spot_prices_state_client::SpotPricesStateClient;
use serde::de::DeserializeOwned;
use std::error::Error;

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

    pub async fn run(&self) -> Result<(), Box<dyn Error>>
    where
        T: DeserializeOwned + SetDefaults,
    {
        let spot_prices_state = self.config.spot_prices_state_client.read_state()?;

        if let Some(state) = spot_prices_state {
            let config: T = self.config.config_client.read_config_from_file()?;
            let spot_price_planner =
                SpotPricePlanner::new(self.config.config_client.read_planner_config_from_file()?);

            self.config
                .planner_client
                .plan(config, spot_price_planner, state.future_spot_prices)
                .await
        } else {
            Err(Box::<dyn Error>::from(
                "No spot prices state present; run jarvis-spot-price-planner first",
            ))
        }
    }
}

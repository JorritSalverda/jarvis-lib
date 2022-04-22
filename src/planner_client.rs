use crate::model::{SpotPrice, SpotPricePlanner};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::error::Error;

#[async_trait]
pub trait PlannerClient<T: ?Sized> {
    async fn plan(
        &self,
        config: T,
        spot_price_planner: SpotPricePlanner,
        spot_prices: Vec<SpotPrice>,
    ) -> Result<(), Box<dyn Error>>
    where
        T: DeserializeOwned;
}

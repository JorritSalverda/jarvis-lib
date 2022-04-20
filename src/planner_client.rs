use crate::model::{SpotPrice, SpotPricePlanner};
use serde::de::DeserializeOwned;
use std::error::Error;

pub trait PlannerClient<T: ?Sized> {
    fn plan(
        &self,
        config: T,
        spot_price_planner: SpotPricePlanner,
        spot_prices: Vec<SpotPrice>,
    ) -> Result<(), Box<dyn Error>>
    where
        T: DeserializeOwned;
}

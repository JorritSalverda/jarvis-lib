use crate::model::SpotPricesState;
use serde::de::DeserializeOwned;
use std::error::Error;

pub trait PlannerClient<T: ?Sized> {
    fn plan(
        &self,
        config: T,
        spot_prices_state: Option<SpotPricesState>,
    ) -> Result<(), Box<dyn Error>>
    where
        T: DeserializeOwned;
}

use crate::model::SpotPrice;
use serde::de::DeserializeOwned;
use std::error::Error;

pub trait PlannerClient<T: ?Sized> {
    fn plan(&self, config: T, best_spot_prices: Vec<SpotPrice>) -> Result<(), Box<dyn Error>>
    where
        T: DeserializeOwned;
}

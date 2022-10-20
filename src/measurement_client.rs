use crate::model::Measurement;
use serde::de::DeserializeOwned;
use std::error::Error;

pub trait MeasurementClient<T: ?Sized> {
    fn get_measurements(
        &self,
        config: T,
        last_measurements: Option<Vec<Measurement>>,
    ) -> Result<Vec<Measurement>, Box<dyn Error>>
    where
        T: DeserializeOwned;
}

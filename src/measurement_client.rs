use crate::model::Measurement;
use serde::de::DeserializeOwned;
use std::error::Error;

pub trait MeasurementClient<T: ?Sized> {
    fn get_measurement(
        &self,
        config: T,
        last_measurement: Option<Measurement>,
    ) -> Result<Measurement, Box<dyn Error>>
    where
        T: DeserializeOwned;
}

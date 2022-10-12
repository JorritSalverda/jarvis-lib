use std::error::Error;

use crate::config_client::{ConfigClient, SetDefaults};
use crate::measurement_client::MeasurementClient;
use crate::nats_client::NatsClient;
use crate::state_client::StateClient;
use serde::de::DeserializeOwned;

pub struct ExporterServiceConfig<T: ?Sized> {
    config_client: ConfigClient,
    nats_client: NatsClient,
    state_client: StateClient,
    measurement_client: Box<dyn MeasurementClient<T>>,
}

impl<T> ExporterServiceConfig<T> {
    pub fn new(
        config_client: ConfigClient,
        nats_client: NatsClient,
        state_client: StateClient,
        measurement_client: Box<dyn MeasurementClient<T>>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config_client,
            nats_client,
            state_client,
            measurement_client,
        })
    }
}

pub struct ExporterService<T> {
    config: ExporterServiceConfig<T>,
}

impl<T> ExporterService<T> {
    pub fn new(config: ExporterServiceConfig<T>) -> Self {
        Self { config }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + SetDefaults,
    {
        let config: T = self.config.config_client.read_config_from_file()?;

        let last_measurement = self.config.state_client.read_state()?;

        let measurement = self
            .config
            .measurement_client
            .get_measurement(config, last_measurement)?;

        if let Some(measurement) = measurement {
            self.config.nats_client.publish(&measurement)?;
            self.config.state_client.store_state(&measurement).await?;
        }

        Ok(())
    }
}

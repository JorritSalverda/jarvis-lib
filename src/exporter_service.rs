use std::error::Error;

use crate::bigquery_client::BigqueryClient;
use crate::config_client::ConfigClient;
use crate::measurement_client::MeasurementClient;
use crate::state_client::StateClient;
use serde::de::DeserializeOwned;

pub struct ExporterServiceConfig<T: ?Sized> {
    config_client: ConfigClient,
    bigquery_client: BigqueryClient,
    state_client: StateClient,
    measurement_client: Box<dyn MeasurementClient<T>>,
}

impl<T> ExporterServiceConfig<T> {
    pub fn new(
        config_client: ConfigClient,
        bigquery_client: BigqueryClient,
        state_client: StateClient,
        measurement_client: Box<dyn MeasurementClient<T>>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config_client,
            bigquery_client,
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

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        let config: T = self.config.config_client.read_config_from_file()?;

        self.config.bigquery_client.init_table().await?;

        let last_measurement = self.config.state_client.read_state()?;

        let measurement = self
            .config
            .measurement_client
            .get_measurement(config, last_measurement)?;

        self.config
            .bigquery_client
            .insert_measurement(&measurement)
            .await?;

        self.config.state_client.store_state(&measurement).await?;

        Ok(())
    }
}

// func TestRun(t *testing.T) {
// 	t.Run("ReadsConfigFromFile", func(t *testing.T) {

// 		ctx := context.Background()
// 		ctrl := gomock.NewController(t)
// 		defer ctrl.Finish()

// 		configClient := NewMockConfigClient(ctrl)
// 		bigqueryClient := NewMockBigQueryClient(ctrl)
// 		stateClient := NewMockStateClient(ctrl)
// 		modbusClient := NewMockModbusClient(ctrl)

// 		bigqueryInit := true
// 		bigqueryDataset := "dataset"
// 		bigqueryTable := "table"
// 		config := Config{}
// 		measurement := contractsv1.Measurement{}

// 		service, _ := NewExporterService(configClient, bigqueryClient, stateClient, modbusClient)

// 		configClient.EXPECT().ReadConfigFromFile(ctx, gomock.Any()).Return(config, nil)
// 		bigqueryClient.EXPECT().InitBigqueryTable(ctx, bigqueryDataset, bigqueryTable).Return(nil)
// 		stateClient.EXPECT().ReadState(ctx).Return(nil, nil)
// 		modbusClient.EXPECT().GetMeasurement(ctx, config, nil).Return(measurement, nil)
// 		bigqueryClient.EXPECT().InsertMeasurement(ctx, bigqueryDataset, bigqueryTable, measurement).Return(nil)
// 		stateClient.EXPECT().StoreState(ctx, measurement).Return(nil)

// 		// act
// 		err := service.Run(ctx, bigqueryInit, bigqueryDataset, bigqueryTable)

// 		assert.Nil(t, err)
// 	})
// }

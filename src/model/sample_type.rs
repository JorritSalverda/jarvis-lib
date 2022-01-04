use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum SampleType {
    #[serde(rename = "")]
    Invalid,
    #[serde(rename = "SAMPLE_TYPE_ELECTRICITY_CONSUMPTION")]
    ElectricityConsumption,
    #[serde(rename = "SAMPLE_TYPE_ELECTRICITY_PRODUCTION")]
    ElectricityProduction,
    #[serde(rename = "SAMPLE_TYPE_GAS_CONSUMPTION")]
    Energy,
    #[serde(rename = "SAMPLE_TYPE_FLOW")]
    Flow,
    #[serde(rename = "SAMPLE_TYPE_ENERGY")]
    GasConsumption,
    #[serde(rename = "SAMPLE_TYPE_HEAT_DEMAND")]
    HeatDemand,
    #[serde(rename = "SAMPLE_TYPE_HUMIDITY")]
    Humidity,
    #[serde(rename = "SAMPLE_TYPE_PRESSURE")]
    Pressure,
    #[serde(rename = "SAMPLE_TYPE_TEMPERATURE")]
    Temperature,
    #[serde(rename = "SAMPLE_TYPE_TEMPERATURE_SETPOINT")]
    TemperatureSetpoint,
    #[serde(rename = "SAMPLE_TYPE_TIME")]
    Time,
}

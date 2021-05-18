use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    #[serde(rename = "")]
    Invalid,
    #[serde(rename = "ENTITY_TYPE_TARIFF")]
    Tariff,
    #[serde(rename = "ENTITY_TYPE_ZONE")]
    Zone,
    #[serde(rename = "ENTITY_TYPE_DEVICE")]
    Device,
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum MetricType {
    #[serde(rename = "")]
    Invalid,
    #[serde(rename = "METRIC_TYPE_COUNTER")]
    Counter,
    #[serde(rename = "METRIC_TYPE_GAUGE")]
    Gauge,
}

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Measurement {
    pub id: String,
    pub source: String,
    pub location: String,
    pub samples: Vec<Sample>,
    pub measured_at_time: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Sample {
    pub entity_type: EntityType,
    pub entity_name: String,
    pub sample_type: SampleType,
    pub sample_name: String,
    pub metric_type: MetricType,
    pub value: f64,
}

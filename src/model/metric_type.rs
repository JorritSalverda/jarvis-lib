use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum MetricType {
    #[serde(rename = "")]
    Invalid,
    #[serde(rename = "METRIC_TYPE_COUNTER")]
    Counter,
    #[serde(rename = "METRIC_TYPE_GAUGE")]
    Gauge,
}

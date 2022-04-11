use crate::model::{EntityType, MetricType, SampleType};
use serde::{Deserialize, Serialize};

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

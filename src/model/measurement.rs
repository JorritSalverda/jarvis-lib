use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::model::Sample;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Measurement {
    pub id: String,
    pub source: String,
    pub location: String,
    pub samples: Vec<Sample>,
    pub measured_at_time: DateTime<Utc>,
}

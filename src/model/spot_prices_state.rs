use crate::model::spot_price::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpotPricesState {
    pub future_spot_prices: Vec<SpotPrice>,
    pub last_from: DateTime<Utc>,
}

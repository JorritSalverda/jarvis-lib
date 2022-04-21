use chrono::{naive::NaiveTime, DateTime, Utc, Weekday};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpotPriceRequest {
    pub query: String,
    pub variables: SpotPriceRequestVariables,
    pub operation_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpotPriceRequestVariables {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpotPriceResponse {
    pub data: SpotPriceData,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpotPriceData {
    pub market_prices_electricity: Vec<SpotPrice>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpotPrice {
    pub id: Option<String>,
    pub source: Option<String>,
    pub from: DateTime<Utc>,
    pub till: DateTime<Utc>,
    pub market_price: f64,
    pub market_price_tax: f64,
    pub sourcing_markup_price: f64,
    pub energy_tax_price: f64,
}

impl SpotPrice {
    pub fn total_price(&self) -> f64 {
        self.market_price
            + self.market_price_tax
            + self.sourcing_markup_price
            + self.energy_tax_price
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpotPricesState {
    pub future_spot_prices: Vec<SpotPrice>,
    pub last_from: DateTime<Utc>,
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum PlanningStrategy {
    Consecutive,
    Fragmented,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TimeSlot {
    pub from: NaiveTime,
    pub till: NaiveTime,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpotPricePlannerConfig {
    pub planning_strategy: PlanningStrategy,
    pub plannable_local_time_slots: HashMap<Weekday, Vec<TimeSlot>>,
    pub session_duration_in_seconds: Option<u32>,
    pub local_time_zone: String,
}

impl SpotPricePlannerConfig {
    pub fn get_local_time_zone(&self) -> Result<Tz, Box<dyn Error>> {
        Ok(self.local_time_zone.parse::<Tz>()?)
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    env_logger::init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fs;

    #[test]
    fn deserialize_spot_price_response() -> Result<(), Box<dyn Error>> {
        let spot_price_predictions_content = fs::read_to_string("spot_price_predictions.json")?;

        let spot_price_response: SpotPriceResponse =
            serde_json::from_str(&spot_price_predictions_content)?;

        assert_eq!(spot_price_response.data.market_prices_electricity.len(), 24);
        Ok(())
    }
}

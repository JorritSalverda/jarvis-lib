use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

    pub fn duration_seconds(&self) -> i64 {
        (self.till - self.from).num_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::SpotPriceResponse;
    use assert2::{check, let_assert};
    use std::fs;

    #[test]
    fn deserialize_spot_price_response() {
        let_assert!(
            Ok(spot_price_predictions_content) = fs::read_to_string("spot_price_predictions.json")
        );

        let_assert!(
            Ok(SpotPriceResponse { data, .. }) =
                serde_json::from_str(&spot_price_predictions_content)
        );

        check!(data.market_prices_electricity.len() == 24);
    }
}

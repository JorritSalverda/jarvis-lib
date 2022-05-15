use crate::model::spot_price::*;
use chrono::prelude::*;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum PlanningStrategy {
    LowestPrice,
    HighestPrice,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoadProfile {
    pub sections: Vec<LoadProfileSection>,
}

impl LoadProfile {
    pub fn total_duration_seconds(&self) -> i64 {
        self.sections.iter().map(|s| s.duration_seconds).sum()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoadProfileSection {
    pub duration_seconds: i64,
    pub power_draw_watt: f64,
}

impl LoadProfileSection {
    pub fn total_power_draw_watt_seconds(&self) -> f64 {
        self.duration_seconds as f64 * self.power_draw_watt
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlanningRequest {
    pub spot_prices: Vec<SpotPrice>,
    pub load_profile: LoadProfile,
    pub planning_strategy: PlanningStrategy,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlanningResponse {
    pub spot_prices: Vec<SpotPrice>,
    pub load_profile: LoadProfile,
}

impl PlanningResponse {
    pub fn total_price(&self) -> f64 {
        total_price_for_load(&self.spot_prices, &self.load_profile)
    }
}

fn total_price_for_load(spot_prices: &[SpotPrice], load_profile: &LoadProfile) -> f64 {
    if !spot_prices.is_empty() && !load_profile.sections.is_empty() {
        let total_required_seconds = load_profile.total_duration_seconds() as usize;

        let mut spot_price_per_second: Vec<f64> = vec![];
        for spot_price in spot_prices {
            let price_per_second = spot_price.total_price() / (3600_f64 * 1000_f64);

            let seconds_still_needed = std::cmp::min(
                spot_price.duration_seconds() as usize,
                total_required_seconds - spot_price_per_second.len(),
            );
            spot_price_per_second.append(&mut vec![price_per_second; seconds_still_needed]);
        }
        assert_eq!(spot_price_per_second.len(), total_required_seconds);

        let mut power_draw_per_second: Vec<f64> = vec![];
        for section in &load_profile.sections {
            power_draw_per_second.append(&mut vec![
                section.power_draw_watt;
                section.duration_seconds as usize
            ]);
        }
        assert_eq!(power_draw_per_second.len(), total_required_seconds);

        // dot product of each vector item
        spot_price_per_second
            .iter()
            .zip(power_draw_per_second.iter())
            .map(|(x, y)| x * y)
            .sum()
    } else {
        0.0
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpotPricePlannerConfig {
    pub load_profile: LoadProfile,
}

pub struct SpotPricePlanner {
    pub config: SpotPricePlannerConfig,
}

impl SpotPricePlanner {
    pub fn new(config: SpotPricePlannerConfig) -> Self {
        Self { config }
    }

    pub fn get_plannable_spot_prices(
        &self,
        spot_prices: &[SpotPrice],
        after: &Option<DateTime<Utc>>,
        before: &Option<DateTime<Utc>>,
    ) -> Result<Vec<SpotPrice>, Box<dyn Error>> {
        info!(
            "Determining plannable spot prices after {:?} and before {:?}",
            after, before
        );
        debug!("spot_prices:\n{:?}", spot_prices);

        let plannable_spot_prices = spot_prices
            .iter()
            .filter(|&spot_price| {
                if let Some(a) = after {
                    if spot_price.from < *a {
                        return false;
                    }
                }

                if let Some(b) = before {
                    if spot_price.till > *b {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        debug!("plannable_spot_prices:\n{:?}", plannable_spot_prices);

        Ok(plannable_spot_prices)
    }

    pub fn get_best_spot_prices(
        &self,
        request: &PlanningRequest,
    ) -> Result<PlanningResponse, Box<dyn Error>> {
        let plannable_spot_prices: Vec<SpotPrice> =
            self.get_plannable_spot_prices(&request.spot_prices, &request.after, &request.before)?;

        if !plannable_spot_prices.is_empty() {
            let total_required_seconds = request.load_profile.total_duration_seconds();
            let mut best_spot_prices: Vec<SpotPrice> = vec![];

            // loop spot prices
            let mut spot_prices_iter = request.spot_prices.iter();
            while let Some(spot_price) = spot_prices_iter.next() {
                let mut selected_spot_prices: Vec<SpotPrice> = vec![spot_price.clone()];
                let mut selected_seconds = spot_price.duration_seconds();

                let mut look_ahead_iter = spot_prices_iter.clone();

                // peek enough consecutive prices to reach total seconds for profile
                while selected_seconds < total_required_seconds {
                    if let Some(next_spot_price) = look_ahead_iter.next() {
                        selected_seconds += next_spot_price.duration_seconds();
                        selected_spot_prices.push(next_spot_price.clone());
                    } else {
                        break;
                    }
                }

                // not enough remaining spot prices to get to the required seconds
                if selected_seconds < total_required_seconds {
                    break;
                }

                if best_spot_prices.is_empty() {
                    // first one, so most applicable yet
                    best_spot_prices = selected_spot_prices;
                } else {
                    // compare to previous best/worst
                    let total_price_previous =
                        total_price_for_load(&best_spot_prices, &request.load_profile);
                    let total_price_current =
                        total_price_for_load(&selected_spot_prices, &request.load_profile);

                    match request.planning_strategy {
                        PlanningStrategy::LowestPrice => {
                            if total_price_current < total_price_previous {
                                best_spot_prices = selected_spot_prices;
                            }
                        }
                        PlanningStrategy::HighestPrice => {
                            if total_price_current > total_price_previous {
                                best_spot_prices = selected_spot_prices;
                            }
                        }
                    }
                }
            }

            Ok(PlanningResponse {
                spot_prices: best_spot_prices,
                load_profile: request.load_profile.clone(),
            })
        } else {
            Ok(PlanningResponse {
                spot_prices: plannable_spot_prices,
                load_profile: request.load_profile.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_price_for_load_returns_zero_for_empty_spot_prices() {
        // act
        let total_price = total_price_for_load(
            &vec![],
            &LoadProfile {
                sections: vec![LoadProfileSection {
                    duration_seconds: 7200,
                    power_draw_watt: 2000.0,
                }],
            },
        );

        assert_eq!(total_price, 0.0);
    }

    #[test]
    fn total_price_for_load_returns_zero_for_empty_load_profile() {
        // act
        let total_price = total_price_for_load(
            &vec![SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(11, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(12, 0, 0),
                market_price: 0.202,
                market_price_tax: 0.0424053,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            }],
            &LoadProfile { sections: vec![] },
        );

        assert_eq!(total_price, 0.0);
    }

    #[test]
    fn total_price_for_load_returns_total_draw_times_total_price_for_equal_length_spot_price_and_load_profile_section(
    ) {
        // act
        let total_price = total_price_for_load(
            &vec![SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(11, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(12, 0, 0),
                market_price: 0.202,
                market_price_tax: 0.0424053,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            }],
            &LoadProfile {
                sections: vec![LoadProfileSection {
                    duration_seconds: 3600,
                    power_draw_watt: 2000.0,
                }],
            },
        );

        assert_eq!(total_price, 0.6848106000000072); // round error, should be 0.6848106
    }

    #[test]
    fn total_price_for_load_returns_total_draw_times_total_price_for_more_spot_prices_than_needed()
    {
        // act
        let total_price = total_price_for_load(
            &vec![
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 14).and_hms(11, 0, 0),
                    till: Utc.ymd(2022, 4, 14).and_hms(12, 0, 0),
                    market_price: 0.202,
                    market_price_tax: 0.0424053,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 14).and_hms(12, 0, 0),
                    till: Utc.ymd(2022, 4, 14).and_hms(13, 0, 0),
                    market_price: 0.195,
                    market_price_tax: 0.0409899,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
            ],
            &LoadProfile {
                sections: vec![
                    LoadProfileSection {
                        duration_seconds: 3600,
                        power_draw_watt: 2000.0,
                    },
                    LoadProfileSection {
                        duration_seconds: 1800,
                        power_draw_watt: 8000.0,
                    },
                ],
            },
        );

        assert_eq!(total_price, 2.0207701999998684); // round error, should be 2.0207702
    }

    #[tokio::test]
    async fn get_best_spot_prices_returns_cheapest_combined_block_spot_of_prices_amounting_to_enough_duration_ordered_by_time_for_lowest_price_strategy(
    ) -> Result<(), Box<dyn Error>> {
        let load_profile = LoadProfile {
            sections: vec![LoadProfileSection {
                duration_seconds: 18000,
                power_draw_watt: 2000.0,
            }],
        };

        let spot_price_planner = SpotPricePlanner::new(SpotPricePlannerConfig {
            load_profile: load_profile.clone(),
        });

        let request = PlanningRequest {
            spot_prices: vec![
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(5, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(6, 0, 0),
                    market_price: 0.189,
                    market_price_tax: 0.03968579999999999,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(6, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(7, 0, 0),
                    market_price: 0.191,
                    market_price_tax: 0.0401352,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(7, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(8, 0, 0),
                    market_price: 0.19,
                    market_price_tax: 0.039816,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(8, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(9, 0, 0),
                    market_price: 0.173,
                    market_price_tax: 0.0362502,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(9, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(10, 0, 0),
                    market_price: 0.147,
                    market_price_tax: 0.030781800000000005,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(10, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(11, 0, 0),
                    market_price: 0.122,
                    market_price_tax: 0.0256179,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(11, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(12, 0, 0),
                    market_price: 0.069,
                    market_price_tax: 0.0145446,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(12, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(13, 0, 0),
                    market_price: 0.025,
                    market_price_tax: 0.0052605,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(13, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(14, 0, 0),
                    market_price: 0.027,
                    market_price_tax: 0.0056364,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(14, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(15, 0, 0),
                    market_price: 0.04,
                    market_price_tax: 0.0084672,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(15, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(16, 0, 0),
                    market_price: 0.066,
                    market_price_tax: 0.013826400000000004,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(16, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(17, 0, 0),
                    market_price: 0.108,
                    market_price_tax: 0.0226191,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(17, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(18, 0, 0),
                    market_price: 0.171,
                    market_price_tax: 0.0359499,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(18, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(19, 0, 0),
                    market_price: 0.195,
                    market_price_tax: 0.0409668,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(19, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(20, 0, 0),
                    market_price: 0.206,
                    market_price_tax: 0.0432201,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(20, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(21, 0, 0),
                    market_price: 0.194,
                    market_price_tax: 0.0408387,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(21, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(22, 0, 0),
                    market_price: 0.176,
                    market_price_tax: 0.0369264,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(22, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(23, 0, 0),
                    market_price: 0.167,
                    market_price_tax: 0.0350448,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
            ],
            load_profile: load_profile,
            planning_strategy: PlanningStrategy::LowestPrice,
            after: None,
            before: None,
        };

        // act
        let response = spot_price_planner.get_best_spot_prices(&request)?;

        assert_eq!(response.total_price(), 1.5294701999999742);

        assert_eq!(response.spot_prices.len(), 5);
        assert_eq!(
            response.spot_prices[0].from,
            Utc.ymd(2022, 4, 16).and_hms(11, 0, 0)
        );
        assert_eq!(response.spot_prices[0].market_price, 0.069);

        assert_eq!(
            response.spot_prices[1].from,
            Utc.ymd(2022, 4, 16).and_hms(12, 0, 0)
        );
        assert_eq!(response.spot_prices[1].market_price, 0.025);

        assert_eq!(
            response.spot_prices[2].from,
            Utc.ymd(2022, 4, 16).and_hms(13, 0, 0)
        );
        assert_eq!(response.spot_prices[2].market_price, 0.027);

        assert_eq!(
            response.spot_prices[3].from,
            Utc.ymd(2022, 4, 16).and_hms(14, 0, 0)
        );
        assert_eq!(response.spot_prices[3].market_price, 0.04);

        assert_eq!(
            response.spot_prices[4].from,
            Utc.ymd(2022, 4, 16).and_hms(15, 0, 0)
        );
        assert_eq!(response.spot_prices[4].market_price, 0.066);

        Ok(())
    }

    #[tokio::test]
    async fn get_best_spot_prices_returns_most_expensive_combined_block_spot_of_prices_amounting_to_enough_duration_ordered_by_time_for_highest_price_strategy(
    ) -> Result<(), Box<dyn Error>> {
        let load_profile = LoadProfile {
            sections: vec![
                LoadProfileSection {
                    duration_seconds: 7200,
                    power_draw_watt: 2000.0,
                },
                LoadProfileSection {
                    duration_seconds: 1800,
                    power_draw_watt: 8000.0,
                },
            ],
        };

        let spot_price_planner = SpotPricePlanner::new(SpotPricePlannerConfig {
            load_profile: load_profile.clone(),
        });

        let request = PlanningRequest {
            spot_prices: vec![
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(5, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(6, 0, 0),
                    market_price: 0.189,
                    market_price_tax: 0.03968579999999999,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(6, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(7, 0, 0),
                    market_price: 0.191,
                    market_price_tax: 0.0401352,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(7, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(8, 0, 0),
                    market_price: 0.19,
                    market_price_tax: 0.039816,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(8, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(9, 0, 0),
                    market_price: 0.173,
                    market_price_tax: 0.0362502,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(9, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(10, 0, 0),
                    market_price: 0.147,
                    market_price_tax: 0.030781800000000005,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(10, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(11, 0, 0),
                    market_price: 0.122,
                    market_price_tax: 0.0256179,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(11, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(12, 0, 0),
                    market_price: 0.069,
                    market_price_tax: 0.0145446,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(12, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(13, 0, 0),
                    market_price: 0.025,
                    market_price_tax: 0.0052605,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(13, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(14, 0, 0),
                    market_price: 0.027,
                    market_price_tax: 0.0056364,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(14, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(15, 0, 0),
                    market_price: 0.04,
                    market_price_tax: 0.0084672,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(15, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(16, 0, 0),
                    market_price: 0.066,
                    market_price_tax: 0.013826400000000004,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(16, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(17, 0, 0),
                    market_price: 0.108,
                    market_price_tax: 0.0226191,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(17, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(18, 0, 0),
                    market_price: 0.171,
                    market_price_tax: 0.0359499,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(18, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(19, 0, 0),
                    market_price: 0.195,
                    market_price_tax: 0.0409668,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(19, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(20, 0, 0),
                    market_price: 0.206,
                    market_price_tax: 0.0432201,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(20, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(21, 0, 0),
                    market_price: 0.194,
                    market_price_tax: 0.0408387,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(21, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(22, 0, 0),
                    market_price: 0.176,
                    market_price_tax: 0.0369264,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
                SpotPrice {
                    id: None,
                    source: None,
                    from: Utc.ymd(2022, 4, 16).and_hms(22, 0, 0),
                    till: Utc.ymd(2022, 4, 16).and_hms(23, 0, 0),
                    market_price: 0.167,
                    market_price_tax: 0.0350448,
                    sourcing_markup_price: 0.017,
                    energy_tax_price: 0.081,
                },
            ],
            load_profile: load_profile,
            planning_strategy: PlanningStrategy::HighestPrice,
            after: None,
            before: None,
        };

        // act
        let response = spot_price_planner.get_best_spot_prices(&request)?;

        assert_eq!(response.total_price(), 2.693728600000162);

        assert_eq!(response.spot_prices.len(), 3);
        assert_eq!(
            response.spot_prices[0].from,
            Utc.ymd(2022, 4, 16).and_hms(18, 0, 0)
        );
        assert_eq!(response.spot_prices[0].market_price, 0.195);

        assert_eq!(
            response.spot_prices[1].from,
            Utc.ymd(2022, 4, 16).and_hms(19, 0, 0)
        );
        assert_eq!(response.spot_prices[1].market_price, 0.206);

        assert_eq!(
            response.spot_prices[2].from,
            Utc.ymd(2022, 4, 16).and_hms(20, 0, 0)
        );
        assert_eq!(response.spot_prices[2].market_price, 0.194);

        Ok(())
    }

    // 2*0.171 + 2*0.195 + 4*0.206 = 1.556
    // 2*0.195 + 2*0.206 + 4*0.194 = 1,578
}

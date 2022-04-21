use crate::model::spot_price::*;
use chrono::{prelude::*, Duration};
use log::{debug, info};
use std::error::Error;

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
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<SpotPrice>, Box<dyn Error>> {
        let local_time_zone = self.config.get_local_time_zone()?;

        info!(
            "Determining plannable spot prices after {:?} and before {:?}",
            after, before
        );
        debug!("spot_prices:\n{:?}", spot_prices);

        let plannable_spot_prices = spot_prices
            .iter()
            .filter(|&spot_price| {
                let local_from = spot_price.from.with_timezone(&local_time_zone);
                let local_till = spot_price.till.with_timezone(&local_time_zone);

                if let Some(a) = after {
                    if spot_price.from < a {
                        return false;
                    }
                }

                if let Some(b) = before {
                    if spot_price.till > b {
                        return false;
                    }
                }

                if let Some(plannable_local_time_slots) = self
                    .config
                    .plannable_local_time_slots
                    .get(&local_from.weekday())
                {
                    plannable_local_time_slots.iter().any(|time_slot| {
                        let time_slot_from = local_from.date().and_hms(
                            time_slot.from.hour(),
                            time_slot.from.minute(),
                            time_slot.from.second(),
                        );

                        let time_slot_till = if time_slot.till.hour() > 0 {
                            local_from.date().and_hms(
                                time_slot.till.hour(),
                                time_slot.till.minute(),
                                time_slot.till.second(),
                            )
                        } else {
                            local_from.date().and_hms(
                                time_slot.till.hour(),
                                time_slot.till.minute(),
                                time_slot.till.second(),
                            ) + Duration::days(1)
                        };

                        local_from >= time_slot_from
                            && local_from < time_slot_till
                            && local_till > time_slot_from
                            && local_till <= time_slot_till
                    })
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        debug!("plannable_spot_prices:\n{:?}", plannable_spot_prices);

        Ok(plannable_spot_prices)
    }

    pub fn get_best_spot_prices(
        &self,
        spot_prices: &[SpotPrice],
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<SpotPrice>, Box<dyn Error>> {
        let mut plannable_spot_prices: Vec<SpotPrice> =
            self.get_plannable_spot_prices(spot_prices, after, before)?;

        match self.config.planning_strategy {
            PlanningStrategy::Fragmented => {
                // sort from lowest prices to highest
                plannable_spot_prices
                    .sort_by(|a, b| a.total_price().partial_cmp(&b.total_price()).unwrap());

                if let Some(seconds) = self.config.session_duration_in_seconds {
                    info!(
                        "Determining best spot prices with strategy {:?} and session duration {}s",
                        self.config.planning_strategy, seconds
                    );

                    // get enough spot prices for session duration
                    let mut spot_price_duration_selected: i64 = 0;
                    let mut selected_spot_prices: Vec<SpotPrice> = vec![];
                    for spot_price in plannable_spot_prices.into_iter() {
                        if spot_price_duration_selected < i64::from(seconds) {
                            let spot_price_duration = spot_price.till - spot_price.from;

                            spot_price_duration_selected += spot_price_duration.num_seconds();
                            selected_spot_prices.push(spot_price);
                        }
                    }

                    // sort by time
                    selected_spot_prices.sort_by(|a, b| a.from.cmp(&b.from));

                    debug!("selected_spot_prices:\n{:?}", selected_spot_prices);

                    Ok(selected_spot_prices)
                } else {
                    Ok(plannable_spot_prices)
                }
            }
            PlanningStrategy::Consecutive => {
                // pick consecutive spot prices that together have lowest price
                if let Some(seconds) = self.config.session_duration_in_seconds {
                    info!(
                        "Determining best spot prices with strategy {:?} and session duration {}s",
                        self.config.planning_strategy, seconds
                    );

                    // get shortest interval to calculate number of slots required when windowing
                    let smallest_interval_in_seconds: i64 = plannable_spot_prices
                        .iter()
                        .map(|sp| (sp.till - sp.from).num_seconds())
                        .min()
                        .unwrap();

                    let window_size =
                        (seconds as f64 / smallest_interval_in_seconds as f64).ceil() as usize;

                    debug!(
                        "Windowing per {} slots for session of {}s due to smallest slot of {}s",
                        window_size, seconds, smallest_interval_in_seconds
                    );

                    let mut windows: Vec<Vec<SpotPrice>> = plannable_spot_prices
                        .windows(window_size)
                        .map(|window| {
                            let mut spot_price_duration_selected: i64 = 0;
                            window
                                .iter()
                                .filter(|sp| {
                                    spot_price_duration_selected +=
                                        (sp.till - sp.from).num_seconds();
                                    spot_price_duration_selected <= i64::from(seconds)
                                })
                                .cloned()
                                .collect::<Vec<SpotPrice>>()
                        })
                        .collect();

                    // sort from lowest prices to highest
                    windows.sort_by(|a, b| {
                        let sum_a: f64 = a.iter().map(|sp| sp.total_price()).sum::<f64>();
                        let sum_b: &f64 = &b.iter().map(|sp| sp.total_price()).sum::<f64>();

                        sum_a.partial_cmp(sum_b).unwrap()
                    });

                    let selected_spot_prices: Vec<SpotPrice> = windows.first().unwrap().to_vec();
                    debug!("selected_spot_prices:\n{:?}", selected_spot_prices);

                    Ok(selected_spot_prices)
                } else {
                    Ok(plannable_spot_prices)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn get_plannable_spot_prices_returns_only_spot_prices_fitting_in_plannable_time_slots(
    ) -> Result<(), Box<dyn Error>> {
        let spot_price_planner = SpotPricePlanner::new(SpotPricePlannerConfig {
            planning_strategy: PlanningStrategy::Fragmented,
            plannable_local_time_slots: HashMap::from([(
                Weekday::Thu,
                vec![TimeSlot {
                    from: NaiveTime::from_hms(14, 0, 0),
                    till: NaiveTime::from_hms(16, 0, 0),
                }],
            )]),
            session_duration_in_seconds: Some(7200),
            local_time_zone: "Europe/Amsterdam".to_string(),
        });

        let future_spot_prices: Vec<SpotPrice> = vec![
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
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(13, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(14, 0, 0),
                market_price: 0.194,
                market_price_tax: 0.0406644,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(14, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(15, 0, 0),
                market_price: 0.192,
                market_price_tax: 0.0403179,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
        ];

        // act
        let plannable_spot_prices =
            spot_price_planner.get_plannable_spot_prices(&future_spot_prices, None, None)?;

        assert_eq!(plannable_spot_prices.len(), 2);
        assert_eq!(
            plannable_spot_prices[0].from,
            Utc.ymd(2022, 4, 14).and_hms(12, 0, 0)
        );
        assert_eq!(
            plannable_spot_prices[0].till,
            Utc.ymd(2022, 4, 14).and_hms(13, 0, 0)
        );
        assert_eq!(
            plannable_spot_prices[1].from,
            Utc.ymd(2022, 4, 14).and_hms(13, 0, 0)
        );
        assert_eq!(
            plannable_spot_prices[1].till,
            Utc.ymd(2022, 4, 14).and_hms(14, 0, 0)
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_plannable_spot_prices_returns_only_spot_prices_fitting_in_plannable_time_slots_when_includes_next_day(
    ) -> Result<(), Box<dyn Error>> {
        let spot_price_planner = SpotPricePlanner::new(SpotPricePlannerConfig {
            planning_strategy: PlanningStrategy::Fragmented,
            plannable_local_time_slots: HashMap::from([
                (
                    Weekday::Thu,
                    vec![
                        TimeSlot {
                            from: NaiveTime::from_hms(14, 0, 0),
                            till: NaiveTime::from_hms(16, 0, 0),
                        },
                        TimeSlot {
                            from: NaiveTime::from_hms(23, 0, 0),
                            till: NaiveTime::from_hms(0, 0, 0),
                        },
                    ],
                ),
                (
                    Weekday::Fri,
                    vec![TimeSlot {
                        from: NaiveTime::from_hms(0, 0, 0),
                        till: NaiveTime::from_hms(2, 0, 0),
                    }],
                ),
            ]),
            session_duration_in_seconds: Some(7200),
            local_time_zone: "Europe/Amsterdam".to_string(),
        });

        let future_spot_prices: Vec<SpotPrice> = vec![
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(20, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(21, 0, 0),
                market_price: 0.265,
                market_price_tax: 0.0557466,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(21, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(22, 0, 0),
                market_price: 0.254,
                market_price_tax: 0.0532728,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(22, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(23, 0, 0),
                market_price: 0.231,
                market_price_tax: 0.0484281,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(23, 0, 0),
                till: Utc.ymd(2022, 4, 15).and_hms(0, 0, 0),
                market_price: 0.215,
                market_price_tax: 0.045129,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 15).and_hms(0, 0, 0),
                till: Utc.ymd(2022, 4, 15).and_hms(1, 0, 0),
                market_price: 0.217,
                market_price_tax: 0.04557,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 15).and_hms(1, 0, 0),
                till: Utc.ymd(2022, 4, 15).and_hms(2, 0, 0),
                market_price: 0.208,
                market_price_tax: 0.0437535,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
        ];

        // act
        let plannable_spot_prices =
            spot_price_planner.get_plannable_spot_prices(&future_spot_prices, None, None)?;

        assert_eq!(plannable_spot_prices.len(), 3);
        assert_eq!(
            plannable_spot_prices[0].from,
            Utc.ymd(2022, 4, 14).and_hms(21, 0, 0)
        );
        assert_eq!(
            plannable_spot_prices[0].till,
            Utc.ymd(2022, 4, 14).and_hms(22, 0, 0)
        );
        assert_eq!(
            plannable_spot_prices[1].from,
            Utc.ymd(2022, 4, 14).and_hms(22, 0, 0)
        );
        assert_eq!(
            plannable_spot_prices[1].till,
            Utc.ymd(2022, 4, 14).and_hms(23, 0, 0)
        );

        assert_eq!(
            plannable_spot_prices[2].from,
            Utc.ymd(2022, 4, 14).and_hms(23, 0, 0)
        );
        assert_eq!(
            plannable_spot_prices[2].till,
            Utc.ymd(2022, 4, 15).and_hms(0, 0, 0)
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_best_spot_prices_returns_cheapest_spot_prices_amounting_to_enough_duration_ordered_by_time(
    ) -> Result<(), Box<dyn Error>> {
        let spot_price_planner = SpotPricePlanner::new(SpotPricePlannerConfig {
            planning_strategy: PlanningStrategy::Fragmented,
            plannable_local_time_slots: HashMap::from([
                (
                    Weekday::Thu,
                    vec![TimeSlot {
                        from: NaiveTime::from_hms(0, 0, 0),
                        till: NaiveTime::from_hms(0, 0, 0),
                    }],
                ),
                (
                    Weekday::Fri,
                    vec![TimeSlot {
                        from: NaiveTime::from_hms(0, 0, 0),
                        till: NaiveTime::from_hms(0, 0, 0),
                    }],
                ),
            ]),
            session_duration_in_seconds: Some(7200),
            local_time_zone: "Europe/Amsterdam".to_string(),
        });

        let future_spot_prices: Vec<SpotPrice> = vec![
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(20, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(21, 0, 0),
                market_price: 0.265,
                market_price_tax: 0.0557466,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(21, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(22, 0, 0),
                market_price: 0.254,
                market_price_tax: 0.0532728,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(22, 0, 0),
                till: Utc.ymd(2022, 4, 14).and_hms(23, 0, 0),
                market_price: 0.231,
                market_price_tax: 0.0484281,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 14).and_hms(23, 0, 0),
                till: Utc.ymd(2022, 4, 15).and_hms(0, 0, 0),
                market_price: 0.215,
                market_price_tax: 0.045129,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 15).and_hms(0, 0, 0),
                till: Utc.ymd(2022, 4, 15).and_hms(1, 0, 0),
                market_price: 0.217,
                market_price_tax: 0.04557,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 15).and_hms(1, 0, 0),
                till: Utc.ymd(2022, 4, 15).and_hms(2, 0, 0),
                market_price: 0.208,
                market_price_tax: 0.0437535,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
        ];

        let best_spot_prices =
            spot_price_planner.get_best_spot_prices(&future_spot_prices, None, None)?;

        assert_eq!(best_spot_prices.len(), 2);
        assert_eq!(
            best_spot_prices[0].from,
            Utc.ymd(2022, 4, 14).and_hms(23, 0, 0)
        );
        assert_eq!(
            best_spot_prices[0].till,
            Utc.ymd(2022, 4, 15).and_hms(0, 0, 0)
        );
        assert_eq!(best_spot_prices[0].market_price, 0.215);

        assert_eq!(
            best_spot_prices[1].from,
            Utc.ymd(2022, 4, 15).and_hms(1, 0, 0)
        );
        assert_eq!(
            best_spot_prices[1].till,
            Utc.ymd(2022, 4, 15).and_hms(2, 0, 0)
        );
        assert_eq!(best_spot_prices[1].market_price, 0.208);

        Ok(())
    }

    #[tokio::test]
    async fn get_best_spot_prices_returns_cheapest_combined_block_spot_of_prices_amounting_to_enough_duration_ordered_by_time_for_consecutive_strategy(
    ) -> Result<(), Box<dyn Error>> {
        let spot_price_planner = SpotPricePlanner::new(SpotPricePlannerConfig {
            planning_strategy: PlanningStrategy::Consecutive,
            plannable_local_time_slots: HashMap::from([(
                Weekday::Sat,
                vec![TimeSlot {
                    from: NaiveTime::from_hms(0, 0, 0),
                    till: NaiveTime::from_hms(0, 0, 0),
                }],
            )]),
            session_duration_in_seconds: Some(18000),
            local_time_zone: "Europe/Amsterdam".to_string(),
        });

        let future_spot_prices: Vec<SpotPrice> = vec![
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
        ];

        // act
        let best_spot_prices =
            spot_price_planner.get_best_spot_prices(&future_spot_prices, None, None)?;

        assert_eq!(best_spot_prices.len(), 5);

        assert_eq!(
            best_spot_prices[0].from,
            Utc.ymd(2022, 4, 16).and_hms(11, 0, 0)
        );
        assert_eq!(best_spot_prices[0].market_price, 0.069);

        assert_eq!(
            best_spot_prices[1].from,
            Utc.ymd(2022, 4, 16).and_hms(12, 0, 0)
        );
        assert_eq!(best_spot_prices[1].market_price, 0.025);

        assert_eq!(
            best_spot_prices[2].from,
            Utc.ymd(2022, 4, 16).and_hms(13, 0, 0)
        );
        assert_eq!(best_spot_prices[2].market_price, 0.027);

        assert_eq!(
            best_spot_prices[3].from,
            Utc.ymd(2022, 4, 16).and_hms(14, 0, 0)
        );
        assert_eq!(best_spot_prices[3].market_price, 0.04);

        assert_eq!(
            best_spot_prices[4].from,
            Utc.ymd(2022, 4, 16).and_hms(15, 0, 0)
        );
        assert_eq!(best_spot_prices[4].market_price, 0.066);

        Ok(())
    }

    #[tokio::test]
    async fn get_plannable_spot_prices_with_before() -> Result<(), Box<dyn Error>> {
        let spot_price_planner = SpotPricePlanner::new(SpotPricePlannerConfig {
            planning_strategy: PlanningStrategy::Consecutive,
            plannable_local_time_slots: HashMap::from([
                (
                    Weekday::Thu,
                    vec![
                        TimeSlot {
                            from: NaiveTime::from_hms(0, 0, 0),
                            till: NaiveTime::from_hms(7, 0, 0),
                        },
                        TimeSlot {
                            from: NaiveTime::from_hms(23, 0, 0),
                            till: NaiveTime::from_hms(0, 0, 0),
                        },
                    ],
                ),
                (
                    Weekday::Fri,
                    vec![
                        TimeSlot {
                            from: NaiveTime::from_hms(0, 0, 0),
                            till: NaiveTime::from_hms(7, 0, 0),
                        },
                        TimeSlot {
                            from: NaiveTime::from_hms(23, 0, 0),
                            till: NaiveTime::from_hms(0, 0, 0),
                        },
                    ],
                ),
            ]),
            session_duration_in_seconds: Some(7200),
            local_time_zone: "Europe/Amsterdam".to_string(),
        });

        let future_spot_prices: Vec<SpotPrice> = vec![
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 21).and_hms(19, 0, 0),
                till: Utc.ymd(2022, 4, 21).and_hms(20, 0, 0),
                market_price: 0.224,
                market_price_tax: 0.0469581,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 21).and_hms(20, 0, 0),
                till: Utc.ymd(2022, 4, 21).and_hms(21, 0, 0),
                market_price: 0.22,
                market_price_tax: 0.0462924,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 21).and_hms(21, 0, 0),
                till: Utc.ymd(2022, 4, 21).and_hms(22, 0, 0),
                market_price: 0.2,
                market_price_tax: 0.0419391,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 21).and_hms(22, 0, 0),
                till: Utc.ymd(2022, 4, 21).and_hms(23, 0, 0),
                market_price: 0.193,
                market_price_tax: 0.040614,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 21).and_hms(23, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(0, 0, 0),
                market_price: 0.206,
                market_price_tax: 0.04326,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(0, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(1, 0, 0),
                market_price: 0.187,
                market_price_tax: 0.0393078,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(1, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(2, 0, 0),
                market_price: 0.187,
                market_price_tax: 0.0392721,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(2, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(3, 0, 0),
                market_price: 0.179,
                market_price_tax: 0.0376761,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(3, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(4, 0, 0),
                market_price: 0.176,
                market_price_tax: 0.0369789,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(4, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(5, 0, 0),
                market_price: 0.19,
                market_price_tax: 0.03981180000000001,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(5, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(6, 0, 0),
                market_price: 0.218,
                market_price_tax: 0.0457947,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(6, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(7, 0, 0),
                market_price: 0.24,
                market_price_tax: 0.0503895,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(7, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(8, 0, 0),
                market_price: 0.244,
                market_price_tax: 0.051260999999999994,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(8, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(9, 0, 0),
                market_price: 0.221,
                market_price_tax: 0.0464205,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(9, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(10, 0, 0),
                market_price: 0.197,
                market_price_tax: 0.0412776,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(10, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(11, 0, 0),
                market_price: 0.157,
                market_price_tax: 0.0330561,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(11, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(12, 0, 0),
                market_price: 0.15,
                market_price_tax: 0.03141599999999999,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(12, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(13, 0, 0),
                market_price: 0.102,
                market_price_tax: 0.02142,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(13, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(14, 0, 0),
                market_price: 0.1,
                market_price_tax: 0.021,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(14, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(15, 0, 0),
                market_price: 0.087,
                market_price_tax: 0.0182217,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(15, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(16, 0, 0),
                market_price: 0.119,
                market_price_tax: 0.0249837,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(16, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(17, 0, 0),
                market_price: 0.167,
                market_price_tax: 0.03507,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(17, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(18, 0, 0),
                market_price: 0.185,
                market_price_tax: 0.038829,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(18, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(19, 0, 0),
                market_price: 0.21,
                market_price_tax: 0.0440181,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(19, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(20, 0, 0),
                market_price: 0.21,
                market_price_tax: 0.0440937,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(20, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(21, 0, 0),
                market_price: 0.21,
                market_price_tax: 0.0440286,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(21, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(22, 0, 0),
                market_price: 0.192,
                market_price_tax: 0.04032,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
            SpotPrice {
                id: None,
                source: None,
                from: Utc.ymd(2022, 4, 22).and_hms(22, 0, 0),
                till: Utc.ymd(2022, 4, 22).and_hms(23, 0, 0),
                market_price: 0.178,
                market_price_tax: 0.0372855,
                sourcing_markup_price: 0.017,
                energy_tax_price: 0.081,
            },
        ];

        // act
        let plannable_spot_prices = spot_price_planner.get_plannable_spot_prices(
            &future_spot_prices,
            Some(Utc.ymd(2022, 4, 21).and_hms(21, 32, 28)),
            Some(Utc.ymd(2022, 4, 22).and_hms(7, 32, 28)),
        )?;

        assert_eq!(plannable_spot_prices.len(), 7);
        assert_eq!(
            plannable_spot_prices[0].from,
            Utc.ymd(2022, 4, 21).and_hms(22, 0, 0)
        );
        assert_eq!(
            plannable_spot_prices[6].till,
            Utc.ymd(2022, 4, 22).and_hms(5, 0, 0)
        );

        Ok(())
    }
}

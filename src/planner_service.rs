use crate::config_client::{ConfigClient, SetDefaults};
use crate::model::*;
use crate::planner_client::PlannerClient;
use crate::spot_prices_state_client::SpotPricesStateClient;
use chrono::{prelude::*, Duration};
use serde::de::DeserializeOwned;
use std::error::Error;

pub struct PlannerServiceConfig<T: ?Sized> {
    config_client: ConfigClient,
    spot_prices_state_client: SpotPricesStateClient,
    planner_client: Box<dyn PlannerClient<T>>,
}

impl<T> PlannerServiceConfig<T> {
    pub fn new(
        config_client: ConfigClient,
        spot_prices_state_client: SpotPricesStateClient,
        planner_client: Box<dyn PlannerClient<T>>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config_client,
            spot_prices_state_client,
            planner_client,
        })
    }
}

pub struct PlannerService<T> {
    config: PlannerServiceConfig<T>,
}

impl<T> PlannerService<T> {
    pub fn new(config: PlannerServiceConfig<T>) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>>
    where
        T: DeserializeOwned + SetDefaults,
    {
        let spot_prices_state = self.config.spot_prices_state_client.read_state()?;

        if let Some(state) = spot_prices_state {
            let planner_config = self.config.config_client.read_planner_config_from_file()?;
            let best_spot_prices =
                get_best_spot_prices(&state.future_spot_prices, &planner_config).await?;

            let config: T = self.config.config_client.read_config_from_file()?;
            self.config.planner_client.plan(config, best_spot_prices)
        } else {
            Err(Box::<dyn Error>::from(
                "No spot prices state present; run jarvis-spot-price-planner first",
            ))
        }
    }
}

pub async fn get_plannable_spot_prices(
    spot_prices: &[SpotPrice],
    planner_config: &SpotPricePlannerConfig,
) -> Result<Vec<SpotPrice>, Box<dyn Error>> {
    let local_time_zone = planner_config.get_local_time_zone()?;

    Ok(spot_prices
        .iter()
        .filter(|&spot_price| {
            let local_from = spot_price.from.with_timezone(&local_time_zone);
            let local_till = spot_price.till.with_timezone(&local_time_zone);

            if let Some(plannable_local_time_slots) = planner_config
                .plannable_local_time_slots
                .get(&local_from.weekday())
            {
                return plannable_local_time_slots.iter().any(|time_slot| {
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
                });
            }

            false
        })
        .cloned()
        .collect())
}

pub async fn get_best_spot_prices(
    spot_prices: &[SpotPrice],
    planner_config: &SpotPricePlannerConfig,
) -> Result<Vec<SpotPrice>, Box<dyn Error>> {
    let mut plannable_spot_prices: Vec<SpotPrice> =
        get_plannable_spot_prices(spot_prices, planner_config).await?;

    match planner_config.planning_strategy {
        PlanningStrategy::Fragmented => {
            // sort from lowest prices to highest
            plannable_spot_prices
                .sort_by(|a, b| a.total_price().partial_cmp(&b.total_price()).unwrap());

            if let Some(mins) = planner_config.session_minutes {
                // get enough spot prices for session duration
                let mut spot_price_duration_selected: i64 = 0;
                let mut selected_spot_prices: Vec<SpotPrice> = vec![];
                for spot_price in plannable_spot_prices.into_iter() {
                    if spot_price_duration_selected < i64::from(mins) {
                        let spot_price_duration = spot_price.till - spot_price.from;

                        spot_price_duration_selected += spot_price_duration.num_minutes();
                        selected_spot_prices.push(spot_price);
                    }
                }

                // sort by time
                selected_spot_prices.sort_by(|a, b| a.from.cmp(&b.from));

                Ok(selected_spot_prices)
            } else {
                Ok(plannable_spot_prices)
            }
        }
        PlanningStrategy::Consecutive => {
            // pick consecutive spot prices that together have lowest price
            if let Some(mins) = planner_config.session_minutes {
                // get shortest interval to calculate number of slots required when windowing
                let smallest_interval_mins: i64 = plannable_spot_prices
                    .iter()
                    .map(|sp| (sp.till - sp.from).num_minutes())
                    .min()
                    .unwrap();
                let required_spot_prices =
                    (mins as f64 / smallest_interval_mins as f64).ceil() as usize;

                let mut windows: Vec<Vec<SpotPrice>> = plannable_spot_prices
                    .windows(required_spot_prices)
                    .map(|window| {
                        let mut spot_price_duration_selected: i64 = 0;
                        window
                            .iter()
                            .filter(|sp| {
                                spot_price_duration_selected += (sp.till - sp.from).num_minutes();
                                spot_price_duration_selected <= i64::from(mins)
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

                Ok(windows.first().unwrap().to_vec())
            } else {
                Ok(plannable_spot_prices)
            }
        }
    }
}

pub async fn get_spot_prices_before(
    spot_prices: &[SpotPrice],
    before: &DateTime<Utc>,
) -> Result<Vec<SpotPrice>, Box<dyn Error>> {
    Ok(spot_prices
        .iter()
        .filter(|&spot_price| spot_price.till <= *before)
        .cloned()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn get_plannable_spot_prices_returns_only_spot_prices_fitting_in_plannable_time_slots(
    ) -> Result<(), Box<dyn Error>> {
        let spot_prices: Vec<SpotPrice> = vec![
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

        let planner_config = SpotPricePlannerConfig {
            planning_strategy: PlanningStrategy::Fragmented,
            plannable_local_time_slots: HashMap::from([(
                Weekday::Thu,
                vec![TimeSlot {
                    from: NaiveTime::from_hms(14, 0, 0),
                    till: NaiveTime::from_hms(16, 0, 0),
                }],
            )]),
            session_minutes: Some(120),
            local_time_zone: "Europe/Amsterdam".to_string(),
        };

        let plannable_spot_prices =
            get_plannable_spot_prices(&spot_prices, &planner_config).await?;

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
        let spot_prices: Vec<SpotPrice> = vec![
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

        let planner_config = SpotPricePlannerConfig {
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
            session_minutes: Some(120),
            local_time_zone: "Europe/Amsterdam".to_string(),
        };

        let plannable_spot_prices =
            get_plannable_spot_prices(&spot_prices, &planner_config).await?;

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
        let spot_prices: Vec<SpotPrice> = vec![
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

        let planner_config = SpotPricePlannerConfig {
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
            session_minutes: Some(120),
            local_time_zone: "Europe/Amsterdam".to_string(),
        };

        let best_spot_prices = get_best_spot_prices(&spot_prices, &planner_config).await?;

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
        let spot_prices: Vec<SpotPrice> = vec![
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

        let planner_config = SpotPricePlannerConfig {
            planning_strategy: PlanningStrategy::Consecutive,
            plannable_local_time_slots: HashMap::from([(
                Weekday::Sat,
                vec![TimeSlot {
                    from: NaiveTime::from_hms(0, 0, 0),
                    till: NaiveTime::from_hms(0, 0, 0),
                }],
            )]),
            session_minutes: Some(300),
            local_time_zone: "Europe/Amsterdam".to_string(),
        };

        let best_spot_prices = get_best_spot_prices(&spot_prices, &planner_config).await?;

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
}

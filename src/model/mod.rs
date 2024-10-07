mod entity_type;
mod measurement;
mod metric_type;
mod sample;
mod sample_type;
mod spot_price;
mod spot_price_planner;
mod spot_prices_state;

pub use crate::model::entity_type::EntityType;
pub use crate::model::measurement::Measurement;
pub use crate::model::metric_type::MetricType;
pub use crate::model::sample::Sample;
pub use crate::model::sample_type::SampleType;
pub use crate::model::spot_price::*;
pub use crate::model::spot_price_planner::*;
pub use crate::model::spot_prices_state::*;

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{check, let_assert};
    use chrono::{DateTime, Utc};

    #[cfg(target_os = "linux")]
    macro_rules! test_case {
        ($f:expr) => {
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", $f))
        };
    }

    #[test]
    fn to_json() {
        let_assert!(
            Ok(measurement) = serde_json::to_string_pretty(&Measurement {
                id: "cc6e17bb-fd60-4dde-acc3-0cda7d752acc".into(),
                source: "jarvis-tp-link-hs-110-exporter".into(),
                location: "My Home".into(),
                samples: vec![Sample {
                    entity_type: EntityType::Device,
                    entity_name: "TP-Link HS110".into(),
                    sample_type: SampleType::ElectricityConsumption,
                    sample_name: "Oven".into(),
                    metric_type: MetricType::Counter,
                    value: 9695872800.0,
                }],
                measured_at_time: DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z")
                    .unwrap()
                    .with_timezone(&Utc),
            })
        );

        check!(measurement == test_case!("test-measurement.json").trim());
    }

    #[test]
    fn from_json() {
        let_assert!(
            Ok(Measurement {
                id,
                source,
                location,
                samples,
                measured_at_time,
            }) = serde_json::from_str(test_case!("test-measurement.json"))
        );

        check!(id == "cc6e17bb-fd60-4dde-acc3-0cda7d752acc");
        check!(source == "jarvis-tp-link-hs-110-exporter");
        check!(location == "My Home");
        check!(samples.len() == 1);

        let_assert!([sample, ..] = samples.as_slice());

        check!(sample.entity_type == EntityType::Device);
        check!(sample.entity_name == "TP-Link HS110");
        check!(sample.sample_type == SampleType::ElectricityConsumption);
        check!(sample.sample_name == "Oven");
        check!(sample.metric_type == MetricType::Counter);
        check!(sample.value == 9695872800.0);

        check!(
            measured_at_time
                == DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z")
                    .unwrap()
                    .with_timezone(&Utc)
        );
    }

    #[test]
    fn to_yaml() {
        let_assert!(
            Ok(str) = serde_yaml::to_string(&Measurement {
                id: "cc6e17bb-fd60-4dde-acc3-0cda7d752acc".into(),
                source: "jarvis-tp-link-hs-110-exporter".into(),
                location: "My Home".into(),
                samples: vec![Sample {
                    entity_type: EntityType::Device,
                    entity_name: "TP-Link HS110".into(),
                    sample_type: SampleType::ElectricityConsumption,
                    sample_name: "Oven".into(),
                    metric_type: MetricType::Counter,
                    value: 9695872800.0,
                }],
                measured_at_time: DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z")
                    .unwrap()
                    .with_timezone(&Utc),
            })
        );

        check!(str == test_case!("test-measurement.yaml"));
    }

    #[test]
    fn from_yaml() {
        let_assert!(
            Ok(Measurement {
                id,
                source,
                location,
                samples,
                measured_at_time,
            }) = serde_yaml::from_str(test_case!("test-measurement.yaml"))
        );

        assert_eq!(id, "cc6e17bb-fd60-4dde-acc3-0cda7d752acc");
        assert_eq!(source, "jarvis-tp-link-hs-110-exporter");
        assert_eq!(location, "My Home");
        assert_eq!(samples.len(), 1);

        let_assert!([first, ..] = samples.as_slice());

        assert_eq!(first.entity_type, EntityType::Device);
        assert_eq!(first.entity_name, "TP-Link HS110");
        assert_eq!(first.sample_type, SampleType::ElectricityConsumption);
        assert_eq!(first.sample_name, "Oven");
        assert_eq!(first.metric_type, MetricType::Counter);
        assert_eq!(first.value, 9695872800.0);

        assert_eq!(
            measured_at_time,
            DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z")
                .unwrap()
                .with_timezone(&Utc)
        );
    }
}

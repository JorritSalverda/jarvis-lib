mod entity_type;
mod sample_type;
mod metric_type;
mod sample;
mod measurement;

pub use crate::model::entity_type::EntityType;
pub use crate::model::sample_type::SampleType;
pub use crate::model::metric_type::MetricType;
pub use crate::model::sample::Sample;
pub use crate::model::measurement::Measurement;

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json;
  use serde_yaml;
  use chrono::{DateTime, Utc};

  #[test]
  fn to_json() {
    assert_eq!(
      serde_json::to_string_pretty(&Measurement{
        id: "cc6e17bb-fd60-4dde-acc3-0cda7d752acc".into(),
        source: "jarvis-tp-link-hs-110-exporter".into(),
        location: "My Home".into(),
        samples: vec![
          Sample{
            entity_type: EntityType::Device,
            entity_name: "TP-Link HS110".into(),
            sample_type: SampleType::ElectricityConsumption,
            sample_name: "Oven".into(),
            metric_type: MetricType::Counter,
            value: 9695872800.0
          }
        ],
        measured_at_time: DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z").unwrap().with_timezone(&Utc),
      })
      .unwrap(),
      r#"{
  "Id": "cc6e17bb-fd60-4dde-acc3-0cda7d752acc",
  "Source": "jarvis-tp-link-hs-110-exporter",
  "Location": "My Home",
  "Samples": [
    {
      "EntityType": "ENTITY_TYPE_DEVICE",
      "EntityName": "TP-Link HS110",
      "SampleType": "SAMPLE_TYPE_ELECTRICITY_CONSUMPTION",
      "SampleName": "Oven",
      "MetricType": "METRIC_TYPE_COUNTER",
      "Value": 9695872800.0
    }
  ],
  "MeasuredAtTime": "2021-05-01T05:45:03.043614293Z"
}"#
    );
  }

  #[test]
  fn from_json() {
    let measurement = serde_json::from_str::<Measurement>(
      r#"{
  "Id": "cc6e17bb-fd60-4dde-acc3-0cda7d752acc",
  "Source": "jarvis-tp-link-hs-110-exporter",
  "Location": "My Home",
  "Samples": [
    {
      "EntityType": "ENTITY_TYPE_DEVICE",
      "EntityName": "TP-Link HS110",
      "SampleType": "SAMPLE_TYPE_ELECTRICITY_CONSUMPTION",
      "SampleName": "Oven",
      "MetricType": "METRIC_TYPE_COUNTER",
      "Value": 9695872800.0
    }
  ],
  "MeasuredAtTime": "2021-05-01T05:45:03.043614293Z"
}"#
    )
    .unwrap();

    assert_eq!(measurement.id, "cc6e17bb-fd60-4dde-acc3-0cda7d752acc");
    assert_eq!(measurement.source, "jarvis-tp-link-hs-110-exporter");
    assert_eq!(measurement.location, "My Home");
    assert_eq!(measurement.samples.len(), 1);
    assert_eq!(measurement.samples.get(0).unwrap().entity_type, EntityType::Device);
    assert_eq!(measurement.samples.get(0).unwrap().entity_name, "TP-Link HS110");
    assert_eq!(measurement.samples.get(0).unwrap().sample_type, SampleType::ElectricityConsumption);
    assert_eq!(measurement.samples.get(0).unwrap().sample_name, "Oven");
    assert_eq!(measurement.samples.get(0).unwrap().metric_type, MetricType::Counter);
    assert_eq!(measurement.samples.get(0).unwrap().value, 9695872800.0);
    assert_eq!(measurement.measured_at_time, DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z").unwrap().with_timezone(&Utc));
  }

  #[test]
  fn to_yaml() {
    assert_eq!(
      serde_yaml::to_string(&Measurement{
        id: "cc6e17bb-fd60-4dde-acc3-0cda7d752acc".into(),
        source: "jarvis-tp-link-hs-110-exporter".into(),
        location: "My Home".into(),
        samples: vec![
          Sample{
            entity_type: EntityType::Device,
            entity_name: "TP-Link HS110".into(),
            sample_type: SampleType::ElectricityConsumption,
            sample_name: "Oven".into(),
            metric_type: MetricType::Counter,
            value: 9695872800.0
          }
        ],
        measured_at_time: DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z").unwrap().with_timezone(&Utc),
      })
      .unwrap(),
      r#"---
Id: cc6e17bb-fd60-4dde-acc3-0cda7d752acc
Source: jarvis-tp-link-hs-110-exporter
Location: My Home
Samples:
  - EntityType: ENTITY_TYPE_DEVICE
    EntityName: TP-Link HS110
    SampleType: SAMPLE_TYPE_ELECTRICITY_CONSUMPTION
    SampleName: Oven
    MetricType: METRIC_TYPE_COUNTER
    Value: 9695872800.0
MeasuredAtTime: "2021-05-01T05:45:03.043614293Z"
"#
    );
  }

  #[test]
  fn from_yaml() {
    let measurement = serde_yaml::from_str::<Measurement>(
      r#"---
Id: cc6e17bb-fd60-4dde-acc3-0cda7d752acc
Source: jarvis-tp-link-hs-110-exporter
Location: My Home
Samples:
  - EntityType: ENTITY_TYPE_DEVICE
    EntityName: TP-Link HS110
    SampleType: SAMPLE_TYPE_ELECTRICITY_CONSUMPTION
    SampleName: Oven
    MetricType: METRIC_TYPE_COUNTER
    Value: 9695872800.0
MeasuredAtTime: "2021-05-01T05:45:03.043614293Z"
"#
    )
    .unwrap();

    assert_eq!(measurement.id, "cc6e17bb-fd60-4dde-acc3-0cda7d752acc");
    assert_eq!(measurement.source, "jarvis-tp-link-hs-110-exporter");
    assert_eq!(measurement.location, "My Home");
    assert_eq!(measurement.samples.len(), 1);
    assert_eq!(measurement.samples.get(0).unwrap().entity_type, EntityType::Device);
    assert_eq!(measurement.samples.get(0).unwrap().entity_name, "TP-Link HS110");
    assert_eq!(measurement.samples.get(0).unwrap().sample_type, SampleType::ElectricityConsumption);
    assert_eq!(measurement.samples.get(0).unwrap().sample_name, "Oven");
    assert_eq!(measurement.samples.get(0).unwrap().metric_type, MetricType::Counter);
    assert_eq!(measurement.samples.get(0).unwrap().value, 9695872800.0);
    assert_eq!(measurement.measured_at_time, DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z").unwrap().with_timezone(&Utc));
  }

}


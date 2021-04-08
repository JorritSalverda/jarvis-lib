#[cfg(test)]
use crate::model::{Sample, EntityType, SampleType, MetricType};

#[test]
fn it_works() {
    let sample = Sample {
        entity_type: EntityType::Device,
        entity_name: String::from("solar pv"),
        sample_type: SampleType::ElectricityProduction,
        sample_name: String::from("total production"),
        metric_type: MetricType::Counter,
        value:      54000000.0,
    };

    // act
    let display = format!("{}", sample);

    assert_eq!(display, "54000000 J");
}
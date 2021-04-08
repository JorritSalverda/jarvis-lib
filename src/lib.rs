#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

struct Measurement {
	ID:             String,
	Source:         String,
	Location:       String,
	Samples:        Vec<Sample>,
	MeasuredAtTime: std::time::SystemTime,
}

struct Sample {
	EntityType: EntityType,
	EntityName: String,
	SampleType: SampleType,
	SampleName: String,
	MetricType: MetricType,
	Value:      f64,
}

enum EntityType {
  Invalid,
  Tariff,
  Zone,
  Device
}

enum SampleType {
  Invalid,
  ElectricityConsumption,
  ElectricityProduction,
  GasConsumption,
  Temperature,
  Pressure,
  Flow,
  Humidity,
  Time
}

enum MetricType {
  Invalid,
  Counter,
  Gauge
}
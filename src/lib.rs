use std::fmt;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let sample = crate::Sample {
          entity_type: crate::EntityType::Device,
          entity_name: String::from("solar pv"),
          sample_type: crate::SampleType::ElectricityProduction,
          sample_name: String::from("total production"),
          metric_type: crate::MetricType::Counter,
          value:      54000000.0,
        };

        // act
        let display = format!("{}", sample);

        assert_eq!(display, "54000000 J");
    }
}

pub struct Measurement {
	pub id:             String,
	pub source:         String,
	pub location:       String,
	pub samples:        Vec<Sample>,
	pub measured_at_time: std::time::SystemTime,
}

pub struct Sample {
	pub entity_type: EntityType,
	pub entity_name: String,
	pub sample_type: SampleType,
	pub sample_name: String,
	pub metric_type: MetricType,
	pub value:      f64,
}

impl fmt::Display for Sample {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{} {}", self.value, self.sample_type)
    }
}

pub enum EntityType {
  Invalid,
  Tariff,
  Zone,
  Device
}

pub enum SampleType {
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

impl fmt::Display for SampleType {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
      match self {
        SampleType::Invalid => write!(f, ""),
        SampleType::ElectricityConsumption => write!(f, "J"),
        SampleType::ElectricityProduction => write!(f, "J"),
        SampleType::GasConsumption => write!(f, "m3"),
        SampleType::Temperature => write!(f, "°C"),
        SampleType::Pressure => write!(f, "Pa"),
        SampleType::Flow => write!(f, "m3s−1"),
        SampleType::Humidity => write!(f, "%"),
        SampleType::Time => write!(f, "s"),
      }
    }
}

pub enum MetricType {
  Invalid,
  Counter,
  Gauge
}
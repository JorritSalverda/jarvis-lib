use std::env;
use std::error::Error;
use crate::model::Measurement;

pub struct NatsClientConfig {
  pub host: String,
  pub subject: String,
  pub queue: String
}

impl NatsClientConfig {
  pub async fn new(
    host: String,
    subject: String,
    queue: String
  ) -> Result<Self, Box<dyn Error>> {
      println!(
          "NatsClientConfig::new(host: {}, subject: {}, queue: {})",
          host, subject, queue
      );

      Ok(Self {
          host,
          subject,
          queue
      })
  }

  pub async fn from_env() -> Result<Self, Box<dyn Error>> {
      let host = env::var("NATS_HOST")
          .unwrap_or_else(|_| String::from("jarvis-nats"));
      let subject = env::var("NATS_SUBJECT")
        .unwrap_or_else(|_| String::from("jarvis-measurements"));
      let queue = env::var("NATS_QUEUE")
        .unwrap_or_else(|_| String::from("jarvis-bigquery-sender"));

      Self::new(
          host,
          subject,
          queue
      )
      .await
  }
}

pub struct NatsClient {
  config: NatsClientConfig,
}

impl NatsClient {
  pub fn new(config: NatsClientConfig) -> NatsClient {
    NatsClient { config }
  }

  pub fn queue_subscribe(&self) -> Result<nats::Subscription, Box<dyn Error>> {
    println!("Subscribing to nats subject {} for queue {}", &self.config.subject, &self.config.queue);

    let nc = nats::connect(&self.config.host).unwrap_or_else(|_| panic!("Failed to connect to nats at {}", &self.config.host));
    
    Ok(nc.queue_subscribe(&self.config.subject, &self.config.queue).unwrap_or_else(|_| panic!("Failed to subscribe to nats subject {} for queue {}", &self.config.subject, &self.config.queue)))
  }

  pub fn publish(&self, measurement: &Measurement) -> Result<(), Box<dyn Error>> {
    println!("Publishing measurement to nats subject {}", &self.config.subject);

    let nc = nats::connect(&self.config.host).unwrap_or_else(|_| panic!("Failed to connect to nats at {}", &self.config.host));

    let msg = serde_json::to_vec(measurement).expect("Failed to serialize measurement");

    nc.publish(&self.config.subject, msg).unwrap_or_else(|_| panic!("Failed to publish measurement to nats subject {}", &self.config.subject));

    Ok(())
  }
}
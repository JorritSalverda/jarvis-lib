use crate::model::Measurement;
use std::env;
use std::error::Error;

pub struct NatsClientConfig {
    pub host: String,
    pub subject: String,
    pub queue: String,
}

impl NatsClientConfig {
    pub async fn new(host: String, subject: String, queue: String) -> Result<Self, Box<dyn Error>> {
        println!(
            "NatsClientConfig::new(host: {}, subject: {}, queue: {})",
            host, subject, queue
        );

        Ok(Self {
            host,
            subject,
            queue,
        })
    }

    pub async fn from_env() -> Result<Self, Box<dyn Error>> {
        let host = env::var("NATS_HOST").unwrap_or_else(|_| String::from("jarvis-nats"));
        let subject =
            env::var("NATS_SUBJECT").unwrap_or_else(|_| String::from("jarvis-measurements"));
        let queue =
            env::var("NATS_QUEUE").unwrap_or_else(|_| String::from("jarvis-bigquery-sender"));

        Self::new(host, subject, queue).await
    }
}

pub struct NatsClient {
    config: NatsClientConfig,
    connection: Option<nats::Connection>,
}

impl NatsClient {
    pub fn new(config: NatsClientConfig) -> NatsClient {
        NatsClient {
            config,
            connection: None,
        }
    }

    fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        self.connection = Some(
            nats::connect(&self.config.host)
                .unwrap_or_else(|_| panic!("Failed to connect to nats at {}", &self.config.host)),
        );

        Ok(())
    }

    pub fn queue_subscribe(&mut self) -> Result<nats::Subscription, Box<dyn Error>> {
        println!(
            "Subscribing to nats subject {} for queue {}",
            &self.config.subject, &self.config.queue
        );

        self.connect()?;

        Ok(self
            .connection
            .as_ref()
            .unwrap()
            .queue_subscribe(&self.config.subject, &self.config.queue)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to subscribe to nats subject {} for queue {}",
                    &self.config.subject, &self.config.queue
                )
            }))
    }

    pub fn publish(&mut self, measurement: &Measurement) -> Result<(), Box<dyn Error>> {
        println!(
            "Publishing measurement to nats subject {}",
            &self.config.subject
        );

        self.connect()?;

        let msg = serde_json::to_vec(measurement).expect("Failed to serialize measurement");

        self.connection
            .as_ref()
            .unwrap()
            .publish(&self.config.subject, msg)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to publish measurement to nats subject {}",
                    &self.config.subject
                )
            });

        Ok(())
    }
}

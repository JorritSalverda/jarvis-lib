use crate::model::Measurement;

use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    api::{Api, PostParams},
    Client,
};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

pub struct StateClientConfig {
    kube_client: kube::Client,
    measurement_file_path: String,
    measurement_file_configmap_name: String,
    current_namespace: String,
}

impl StateClientConfig {
    pub fn new(
        kube_client: kube::Client,
        measurement_file_path: String,
        measurement_file_configmap_name: String,
        current_namespace: String,
    ) -> Result<Self, Box<dyn Error>> {
        println!(
            "StateClientConfig::new(measurement_file_path: {}, measurement_file_configmap_name: {}, current_namespace: {})",
            measurement_file_path, measurement_file_configmap_name, current_namespace
        );
        Ok(Self {
            kube_client,
            measurement_file_path,
            measurement_file_configmap_name,
            current_namespace,
        })
    }

    pub async fn from_env() -> Result<Self, Box<dyn Error>> {
        let kube_client: kube::Client = Client::try_default().await?;

        let measurement_file_path = env::var("MEASUREMENT_FILE_PATH")
            .unwrap_or_else(|_| "/configs/last-measurement.yaml".to_string());
        let measurement_file_configmap_name = env::var("MEASUREMENT_FILE_CONFIG_MAP_NAME")
            .unwrap_or_else(|_| "jarvis-modbus-exporter".to_string());

        let current_namespace =
            fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/namespace")?;

        Self::new(
            kube_client,
            measurement_file_path,
            measurement_file_configmap_name,
            current_namespace,
        )
    }
}

pub struct StateClient {
    // kubeClientset                *kubernetes.Clientset
    config: StateClientConfig,
}

impl StateClient {
    pub fn new(config: StateClientConfig) -> StateClient {
        StateClient { config }
    }

    pub fn read_state(&self) -> Result<Option<Measurement>, Box<dyn std::error::Error>> {
        let state_file_contents = match fs::read_to_string(&self.config.measurement_file_path) {
            Ok(c) => c,
            Err(_) => return Ok(Option::None),
        };

        let last_measurement: Option<Measurement> = match serde_yaml::from_str(&state_file_contents)
        {
            Ok(lm) => Some(lm),
            Err(_) => return Ok(Option::None),
        };

        println!(
            "Read previous measurement from state file at {}",
            &self.config.measurement_file_path
        );

        Ok(last_measurement)
    }

    async fn get_state_configmap(&self) -> Result<ConfigMap, Box<dyn std::error::Error>> {
        let configmaps_api: Api<ConfigMap> = Api::namespaced(
            self.config.kube_client.clone(),
            &self.config.current_namespace,
        );

        let config_map = configmaps_api
            .get(&self.config.measurement_file_configmap_name)
            .await?;

        Ok(config_map)
    }

    async fn update_state_configmap(
        &self,
        config_map: &ConfigMap,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let configmaps_api: Api<ConfigMap> = Api::namespaced(
            self.config.kube_client.clone(),
            &self.config.current_namespace,
        );

        configmaps_api
            .replace(
                &self.config.measurement_file_configmap_name,
                &PostParams::default(),
                config_map,
            )
            .await?;

        Ok(())
    }

    pub async fn store_state(
        &self,
        measurement: &Measurement,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // retrieve configmap
        let mut config_map = self.get_state_configmap().await?;

        // marshal state to yaml
        let yaml_data = match serde_yaml::to_string(measurement) {
            Ok(yd) => yd,
            Err(e) => return Err(Box::new(e)),
        };

        // extract filename from config file path
        let measurement_file_path = Path::new(&self.config.measurement_file_path);
        let measurement_file_name = match measurement_file_path.file_name() {
            Some(filename) => match filename.to_str() {
                Some(filename) => String::from(filename),
                None => return Err(Box::<dyn Error>::from("No filename found in path")),
            },
            None => return Err(Box::<dyn Error>::from("No filename found in path")),
        };

        // update data in configmap
        let mut data: std::collections::BTreeMap<String, String> = match config_map.data {
            Some(d) => d,
            None => BTreeMap::new(),
        };
        data.insert(measurement_file_name, yaml_data);
        config_map.data = Some(data);

        // update configmap to have measurement available when the application runs the next time and for other applications
        self.update_state_configmap(&config_map).await?;

        println!(
            "Stored last measurement in configmap {}",
            &self.config.measurement_file_configmap_name
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EntityType,MetricType,SampleType};
    use chrono::DateTime;

    #[tokio::test]
    #[ignore]
    async fn read_measurement_from_file_returns_deserialized_test_file() {
        let kube_client: kube::Client = match Client::try_default().await {
            Ok(c) => c,
            Err(e) => panic!("Getting kube_client errored: {}", e),
        };

        let measurement_file_path = "test-measurement.yaml".to_string();
        let measurement_file_configmap_name = "jarvis-modbus-exporter-sunny".to_string();
        let current_namespace = "jarvis".to_string();

        let state_client = StateClient::new(
            StateClientConfig::new(
                kube_client,
                measurement_file_path,
                measurement_file_configmap_name,
                current_namespace,
            )
            .unwrap(),
        );

        let last_measurement = state_client.read_state().unwrap();
        match last_measurement {
            Some(lm) => {
                assert_eq!(lm.id, "cc6e17bb-fd60-4dde-acc3-0cda7d752acc".to_string());
                assert_eq!(lm.source, "jarvis-modbus-exporter".to_string());
                assert_eq!(lm.location, "My Home".to_string());
                assert_eq!(lm.samples.len(), 1);
                assert_eq!(lm.samples[0].entity_type, EntityType::Device);
                assert_eq!(lm.samples[0].entity_name, "Sunny TriPower 8.0".to_string());
                assert_eq!(lm.samples[0].sample_type, SampleType::ElectricityProduction);
                assert_eq!(lm.samples[0].sample_name, "Total production".to_string());
                assert_eq!(lm.samples[0].metric_type, MetricType::Counter);
                assert_eq!(lm.samples[0].value, 9695872800.0f64);
                assert_eq!(
                    lm.measured_at_time,
                    DateTime::parse_from_rfc3339("2021-05-01T05:45:03.043614293Z").unwrap()
                );
            }
            None => panic!("read_state returned no measurement"),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn get_last_measurement() {
        let kube_client: kube::Client = match Client::try_default().await {
            Ok(c) => c,
            Err(e) => panic!("Getting kube_client errored: {}", e),
        };

        let measurement_file_path = "/configs/last-measurement.yaml".to_string();
        let measurement_file_configmap_name = "jarvis-modbus-exporter-sunny".to_string();
        let current_namespace = "jarvis".to_string();

        let state_client = StateClient::new(
            StateClientConfig::new(
                kube_client,
                measurement_file_path,
                measurement_file_configmap_name,
                current_namespace,
            )
            .unwrap(),
        );

        let config_map = state_client.get_state_configmap().await;

        match config_map {
            Ok(cm) => {
                assert_eq!(cm.data.unwrap().len(), 10);
            }
            Err(e) => panic!("get_state_configmap errored: {}", e),
        }
    }
}

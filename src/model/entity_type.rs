use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    #[serde(rename = "")]
    Invalid,
    #[serde(rename = "ENTITY_TYPE_TARIFF")]
    Tariff,
    #[serde(rename = "ENTITY_TYPE_ZONE")]
    Zone,
    #[serde(rename = "ENTITY_TYPE_DEVICE")]
    Device,
}

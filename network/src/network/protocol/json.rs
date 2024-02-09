use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkNode {
    pub network_id: String,
    pub multiaddr: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkNodeRecord {
    pub nodes: Vec<NetworkNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DecentNetResponse {
    Pong,
    Record(NetworkNodeRecord),
    GotNetworkRecord,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DecentNetRequest {
    Ping,
    GetNetworkNodes,
    SendNodeRecord(NetworkNodeRecord),
}

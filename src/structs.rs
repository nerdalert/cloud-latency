pub const PROTO_ICMP: &str = "icmp";
pub const PROTO_TCP: &str = "tcp";

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Config {
    pub tsdb_prefix: String,
    pub test_interval: u64,
    pub grafana_address: String,
    pub grafana_port: String,
    pub endpoints: Vec<String>,
    pub tcp_endpoints: Vec<String>,
}

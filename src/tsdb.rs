use crate::structs;

use std::io::Write;
use std::net::TcpStream;

// send the stream containing the measurements to the carbon socket
fn write_stream(
    stream: &mut std::net::TcpStream,
    hostname: &str,
    probe_and_value: String,
    timestamp: i64,
) {
    let content = format!("{}.{} {}\n", hostname, probe_and_value, timestamp);
    // println!("Debug: writing tsdb data -> {}", content); // Uncomment to debug carbon writes
    let _ = stream.write(&content.as_bytes());
}

// compose the stream containing the measurements to be sent to the tsdb
pub fn write_tsdb(config: &structs::Config, endp: String, proto: &str, time: std::time::Duration) {
    let dt = chrono::Utc::now();
    let timestamp = dt.timestamp();
    let grafana_svr = format!("{}:{}", config.grafana_address, config.grafana_port);
    let socker_addr: &str = grafana_svr.as_str();
    // replace all "." with "_" to record properly in the tsdb
    let endpoint_name = str::replace(&endp, ".", "-");
    let prefix = str::replace(config.tsdb_prefix.as_str(), ".", "-");
    let tsdb_prefix = format!("{}.{}", prefix, proto);
    match TcpStream::connect(socker_addr) {
        Ok(mut stream) => {
            write_stream(
                &mut stream,
                tsdb_prefix.as_str(),
                format!("{} {}", endpoint_name, time.as_millis()),
                // You can debug with a generic value using:
                // format!("{} 543324500", endpoint_name),
                timestamp,
            );
        }
        Err(e) => println!(
            "Unable to connect to the Graphite server at {}: {:?} dropping this measurement",
            grafana_svr, e
        ),
    }
}

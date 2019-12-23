use dns_lookup::lookup_host;
use futures::Future;
use std::io::prelude::*;
use std::net::IpAddr;
use std::net::TcpStream;

use crate::structs;

// spawn the threads for the async pings for each endpoint
pub fn measure_latency(pinger: tokio_ping::Pinger, config: structs::Config) {
    // Iterate over the targets
    for endp in config.endpoints.iter() {
        let ip_lookup = resolve_host(endp);
        let ip_v4 = ip_lookup.unwrap();
        let endpoint_name = endp.to_string().clone();
        let config_copy = config.clone();
        let ping: tokio_ping::PingFuture = pinger.chain(ip_v4).send();
        tokio::spawn(
            ping.and_then(move |resp| {
                if let Some(delay) = resp {
                    println!(
                        "Target: {:?} Latency: {:?}ms",
                        endpoint_name,
                        delay.as_millis(),
                    );
                    write_tsdb(&config_copy, endpoint_name, delay);
                }
                Ok(())
            })
            .map_err(|e| {
                eprintln!("{:?}", e);
            }),
        );
    }
}

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
fn write_tsdb(config: &structs::Config, endp: String, time: std::time::Duration) {
    let dt = chrono::Utc::now();
    let timestamp = dt.timestamp();
    let grafana_svr = format!("{}:{}", config.grafana_address, config.grafana_port);
    let socker_addr: &str = grafana_svr.as_str();
    // replace all "." with "_" to record properly in the tsdb
    let endpoint_name = str::replace(&endp, ".", "-");
    let tsdb_prefix = str::replace(config.tsdb_prefix.as_str(), ".", "-");

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

// resolve a hostname to IpAddr
pub fn resolve_host(host: &str) -> Result<IpAddr, String> {
    match host.parse::<IpAddr>() {
        Ok(val) => Ok(val),
        _ => {
            let ip_list = lookup_host(host).map_err(|_| "dns_lookup::lookup_host failed")?;
            Ok(*ip_list.first().unwrap())
        }
    }
}

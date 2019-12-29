use crate::structs;
use crate::structs::{PROTO_ICMP, PROTO_TCP};
use crate::tsdb;

use dns_lookup::lookup_host;
use futures::Future;
use std::net::IpAddr;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

// spawn the threads for the async pings for each endpoint
pub fn measure_latency(pinger: tokio_ping::Pinger, config: structs::Config) {
    if config.endpoints.is_empty() {
        // Skip to the tcp monitoring threads if ICMP targets is empty
        measure_tcp_latency(config);
    } else {
        // Iterate over the ICMP targets
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
                            "ICMP Target: {:?} Latency: {:?}ms",
                            endpoint_name,
                            delay.as_millis(),
                        );
                        tsdb::write_tsdb(&config_copy, endpoint_name, PROTO_ICMP, delay);
                    }
                    Ok(())
                })
                .map_err(|e| {
                    eprintln!("{:?}", e);
                }),
            );
        }
        // Call the tcp monitoring threads
        measure_tcp_latency(config);
    }
}

// spawn the threads for the async TCP sockets for each endpoint
pub fn measure_tcp_latency(config: structs::Config) {
    if config.tcp_endpoints.is_empty() {
        return;
    }
    // Iterate over the tcp targets
    for tcp_endpoint in config.tcp_endpoints.iter() {
        let p = String::from(":");
        // Split on the ':' pattern
        if tcp_endpoint.contains(&p) {
            let mut split = tcp_endpoint.split(&p);
            let ip_split = split.next().unwrap();
            let port = split.next().unwrap();
            let ip_lookup = resolve_host(ip_split);
            let ip_v4 = ip_lookup.unwrap();
            let endpoint_name = tcp_endpoint.to_string().clone();
            let join_socket = format!("{}:{}", ip_v4.to_string(), port);
            let sock_addr: SocketAddr =
                join_socket.parse().expect("Unable to parse socket address");
            let config_copy = config.clone();
            let handle = std::thread::spawn(move || {
                let instant_start = Instant::now();
                if TcpStream::connect_timeout(&sock_addr, Duration::new(5, 0)).is_ok() {
                    let instant_end = Instant::now();
                    let delay = instant_end - instant_start;
                    println!(
                        "TCP Target: {:?} Latency: {:?}ms",
                        endpoint_name,
                        delay.as_millis()
                    );
                    // Send the measurement to the TSDB module for formmating and shipping
                    tsdb::write_tsdb(&config_copy, endpoint_name, PROTO_TCP, delay);
                } else {
                    println!("Failed connection to {:?}", sock_addr);
                }
            });
            handle.join().unwrap();
        } else {
            println!("Pattern to split on '{}' not found", p);
        }
    }
}

// resolve a hostname to std::net::IpAddr
pub fn resolve_host(host: &str) -> Result<IpAddr, String> {
    match host.parse::<IpAddr>() {
        Ok(val) => Ok(val),
        _ => {
            let ip_list = lookup_host(host).map_err(|_| "dns_lookup::lookup_host failed")?;
            Ok(*ip_list.first().unwrap())
        }
    }
}

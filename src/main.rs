#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate futures;
extern crate serde;
extern crate serde_yaml;
extern crate tokio;
extern crate tokio_ping;

use clap::{App, Arg};
use futures::{Future, Stream};
use std::fs::File;
use std::io::Read;
use std::net::IpAddr;
use std::path::Path;
use std::process;
use std::time::Duration;
use structs::Config;
use tokio::timer::Interval;

mod networker;
mod structs;
mod tsdb;

const CONFIG: &str = "./config.yml";

fn main() {
    // CLI arg parse
    let args = App::new("cloud-latency")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .hidden_short_help(true)
                .value_name("FILE")
                .help("Path to the config.yml file (defaults to ./config.yml)"),
        )
        .get_matches();
    // Print the values from the user args or the default setter
    let config_location = args.value_of("config").unwrap_or(CONFIG);
    println!("Using configuration file: {}", config_location);

    // If the config.yml file does not exist in the specified location exit(0).
    if !file_exists(config_location) {
        println!(
            "The configuration file was not found in the specified location: {}. Please specify the location with --config=config.yml",
            config_location
        );
        process::exit(0)
    }
    // deserialize configuration data from the yaml file
    let config = get_config(CONFIG);
    let icmp_fut = tokio_ping::Pinger::new()
        .map_err(|e| panic!("{:?}", e))
        .and_then(move |pinger| thread_interval(pinger, config));
    tokio::run(icmp_fut.map_err(|e| panic!("{:?}", e)));
}

// Open and deserialize the yml config file in ./config.yml
fn get_config(filename: &str) -> Config {
    let mut file = File::open(filename).expect("Unable to open the file");
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let config: Config = serde_yaml::from_str(&content).unwrap();
    config
}

// parse a string representation of an IP address to the IpAddr Enum
fn parse_ip_result(txt: &str) -> Result<IpAddr, String> {
    match txt.parse::<IpAddr>() {
        Ok(val) => Ok(val),
        Err(err) => Err(format!(
            "Could not parse IP from {} because of {}",
            txt, err
        )),
    }
}

// verify referenced configuration file exists
fn file_exists(config_file: &str) -> bool {
    Path::new(config_file).exists()
}

// setup the future producer
fn thread_interval(
    pinger: tokio_ping::Pinger,
    config: structs::Config,
) -> Box<dyn Future<Item = (), Error = ()> + Send> {
    println!("Configuration is as follows:");
    println!("- grafana server: {}", config.grafana_address);
    println!("- grafana port: {}", config.grafana_port);
    println!("- test interval (sec): {}", config.test_interval);
    println!("- tsdb prefix: {}", config.tsdb_prefix);
    println!("- ICMP endpoints to be monitored:");
    for name in config.endpoints.iter() {
        println!("{}", name);
    }
    println!("- TCP endpoints to be monitored:");
    for tcp_name in config.tcp_endpoints.iter() {
        println!("{}", tcp_name);
    }
    let interval = Interval::new_interval(Duration::from_secs(config.test_interval));
    Box::new(
        interval
            .for_each(move |_| {
                networker::measure_latency(pinger.clone(), config.clone());
                Ok(())
            })
            .map_err(|e| {
                eprintln!("{:?}", e);
            }),
    )
}

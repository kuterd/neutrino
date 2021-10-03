//extern crate yaml_rust;
extern crate tokio;

use yaml_rust::{YamlLoader, Yaml};
use std::fs;

//use tokio::io::*;

mod general;
mod socks5;

use crate::general::*;
use crate::socks5::socks5_server;

// Initialize all chains.
async fn materialize_chains(config: &Vec<Yaml>) {
    for chain in config {
        let node_type = chain["type"].as_str()
            .expect("Expected a 'type' for chain"); 
        
        println!("Constructing chain node: {:?}", node_type);
        // Create the appropriate type 
        match node_type {
            "socks5_server" => {
                socks5_server(chain).await; 
            }
            _ => {
                panic!("Unknwon chain type");
            }
        } 
    }
}

#[tokio::main]
async fn main() {
    let config_str = fs::read_to_string("config.yaml")
        .expect("Failed to read the config file"); 
    let config = &YamlLoader::load_from_str(&config_str)
        .expect("Failed to parse the config");
    materialize_chains(&config[0].as_vec().expect("expected chain array")).await;
}

//extern crate yaml_rust;
extern crate tokio;

use yaml_rust::YamlLoader;
use std::fs;

//use tokio::io::*;

mod general;
mod socks5;

use crate::general::*;


#[tokio::main]
async fn main() {
    let config_str = fs::read_to_string("config.yaml")
        .expect("Failed to read the config file");
    let config = &YamlLoader::load_from_str(&config_str)
        .expect("Failed to parse the config");
    let chain = ChainSet::load(&config[0][0]).await;
    chain.run().await.unwrap();
}

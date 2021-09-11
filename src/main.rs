//extern crate yaml_rust;
extern crate tokio;

use yaml_rust::{YamlLoader};
use std::fs;

use tokio::io::*;

mod general;
mod socks5;
use crate::general::*;
use crate::socks5::*;

async fn run() {
    let tcp_node = Box::new(TcpChainNode {});
    let node = Socks5ChainNode {parent: tcp_node, proxy_addr: "127.0.0.1:9050".parse().unwrap()};
    let req = LinkRequest {addr: LinkAddr::Domain("google.com".as_bytes().to_vec()), port: 80};   
    let mut conn = node.connect(req).await.unwrap();
    println!("connected"); 
    conn.write_all("GET / HTTP/1.1\r\nHost: google.com\r\n\r\n".as_bytes()).await.unwrap();
    conn.flush().await.unwrap();
    println!("header sent");
    loop {
        let b = conn.read_u8().await;
        if !b.is_ok() {
            break; 
        }
        print!("{}", b.unwrap() as char);
    }
    println!("DONE");
}

// Constructs a chain from a config file.
fn load_chain()  {
    let config_str = fs::read_to_string("config.yaml")
        .expect("Failed to read the config file"); 
    let config = &YamlLoader::load_from_str(&config_str).unwrap()[0];

    //TODO: Allow multiple chains.
    let chain = &config["chain"];
    let mut parent : Option<Box<dyn ChainNode>> = None;

    for node in chain.as_vec().unwrap() {
        let n_type = &node["type"];
        println!("Node type: {}", n_type.as_str().unwrap());
        
        // Consider doing this with some macro magic.
        match n_type.as_str().unwrap() {
            "socks5" => {
                assert!(parent.is_some(), "socks5 node must not be the first node in the chain!");
                let addr = node["addr"].as_str().unwrap().parse().unwrap();
                parent =
                     Some(Box::new(Socks5ChainNode {parent: parent.unwrap(), proxy_addr: addr}));
            }
            "tcp" => {
                assert!(parent.is_none(), "tcp node must be the first node in the chain!"); 
                parent = Some(Box::new(TcpChainNode {}));
            }
            _ => {
                panic!("Unknown node"); 
            }
        }
    } 
}

#[tokio::main]
async fn main() {
    load_chain();

}

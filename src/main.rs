//extern crate yaml_rust;
extern crate tokio;

use yaml_rust::YamlLoader;
use std::fs;

use tokio::io::*;

mod general;
mod socks5;


use crate::general::*;
use crate::socks5::*;


/*
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
*/

#[tokio::main]
async fn main() {
    let config_str = fs::read_to_string("config.yaml")
        .expect("Failed to read the config file"); 
    let config = &YamlLoader::load_from_str(&config_str).unwrap()[0];
    let chain = &config["chain"].as_vec().unwrap();

    load_chain(&chain);
}

use async_trait::async_trait;

use tokio::io::*;
use tokio::net::*;
use std::net::{Ipv4Addr, Ipv6Addr, IpAddr, SocketAddr};
use std::io::Result;
use std::fmt::Debug;
use std::sync::Arc;
use yaml_rust::yaml::Array;

use crate::socks5::*; 

pub trait AsyncRW: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> AsyncRW for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

// Standart information about a chain node.
#[derive(Debug)]
pub struct ChainNodeInfo {
    pub name: Option<String>,
    pub parent: Option<Arc<dyn ChainNode>>
}

// Address of a connection
// std::net::SocketAddr can not represent a domain address.
#[allow(dead_code)]
#[derive(Debug)]
pub enum LinkAddr {
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
    Domain(Vec<u8>)
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct LinkRequest {
    pub addr: LinkAddr,
    pub port: u16  
}

impl LinkRequest {
    // Convert a link request to a socket address.
    async fn as_socket_addr(&self) -> Result<SocketAddr> {
       match &self.addr { 
            LinkAddr::IPv4(ip) => {
                return Ok(SocketAddr::new(IpAddr::V4(*ip), self.port));
            }
            LinkAddr::IPv6(ip) => {
                return Ok(SocketAddr::new(IpAddr::V6(*ip), self.port));
            }
            //TODO: Implement DNS lookups.
            LinkAddr::Domain(_domain) => {
                return Err(Error::new(ErrorKind::InvalidData, "Domain type is not supported for now"));
            }
       };
    } 
}

// Convert a Socket addr to a link request.
impl From<SocketAddr> for LinkRequest {
    fn from(addr: SocketAddr) -> LinkRequest {
        match addr {
            SocketAddr::V4(addr) => {
                return LinkRequest {addr: LinkAddr::IPv4(*addr.ip()), port: addr.port()}; 
            }
            SocketAddr::V6(addr) => {
                return LinkRequest {addr: LinkAddr::IPv6(*addr.ip()), port: addr.port()}; 
            }
        }     
    }
}

#[async_trait]
pub trait ChainNode: Send + Sync + Debug {
    async fn connect(&self, req: LinkRequest) -> Result<Box<dyn AsyncRW>>;
}

#[derive(Debug)]
pub struct TcpChainNode {
    pub node_info : ChainNodeInfo
}

#[async_trait]
impl ChainNode for TcpChainNode {
    async fn connect(&self, req: LinkRequest) -> Result<Box<dyn AsyncRW>> {
       let stream = TcpStream::connect(req.as_socket_addr().await?).await?;
//       Sadly, we can't use a buffered stream here.
//       I will eventually fix this limitation.
//       return Ok(Box::<BufStream<TcpStream>>::new(BufStream::new(stream)));
       return Ok(Box::new(stream));
    }
}

// Constructs a chain from a Yaml array.
pub fn load_chain(chain: &Array) -> Arc<dyn ChainNode> {
    let mut parent : Option<Arc<dyn ChainNode>> = None;
    for node in chain {
        let mut name : Option<String> = None;
        let mname = node["name"].as_str();
        if let Some(st) = mname {
            name = Some(st.to_string());
        }

        let node_info = ChainNodeInfo { name: name,  parent: parent.clone() };

        let n_type = &node["type"];
        println!("Node type: {}", n_type.as_str().unwrap());
        
        // Consider doing this with some macro magic.
        match n_type.as_str().unwrap() {
            "socks5" => {
                assert!(parent.is_some(), "socks5 node must not be the first node in the chain!");
                let addr = node["addr"].as_str().unwrap().parse().unwrap();
                parent =
                     Some(Arc::new(Socks5ChainNode {parent: parent.unwrap(), proxy_addr: addr}));
            }
            "tcp" => {
                assert!(parent.is_none(), "tcp node must be the first node in the chain!"); 
                parent = Some(Arc::new(TcpChainNode {node_info: node_info}));
            }
            _ => {
                panic!("Unknown node"); 
            }
        }
    }
    println!("Chain {:?}", parent);
    return parent.unwrap();
}

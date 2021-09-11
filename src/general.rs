use async_trait::async_trait;

use tokio::io::*;
use tokio::net::*;
use std::net::{Ipv4Addr, Ipv6Addr, IpAddr, SocketAddr};
use std::io::Result;

pub trait AsyncRW: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> AsyncRW for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

// Standart info about a struct.
/*
struct ChainInfo {
    pub name: String,
    pub parent: Option<Box<dyn ChainNode>>
}
*/

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
pub trait ChainNode: Send + Sync {
    async fn connect(&self, req: LinkRequest) -> Result<Box<dyn AsyncRW>>;
}

// This is backed by a buffered stream.
pub struct TcpChainNode {}

#[async_trait]
impl ChainNode for TcpChainNode {
    async fn connect(&self, req: LinkRequest) -> Result<Box<dyn AsyncRW>> {
       let stream = TcpStream::connect(req.as_socket_addr().await?).await?;
//       Sadly, we can't use a buffered stream here.
//       return Ok(Box::<BufStream<TcpStream>>::new(BufStream::new(stream)));
       return Ok(Box::new(stream));
    }
}


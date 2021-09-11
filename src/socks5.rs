use async_trait::async_trait;

use yaml_rust::Yaml;

use tokio::io::*;
use tokio::net::*;
use std::net::{Ipv4Addr, SocketAddr};

use std::io::Result;

use crate::general::*;

#[allow(dead_code)]
#[derive(Debug)]
enum ReplyType {
    Succeeded,
    GeneralFailure,
    Unreachable,
    Refused,
    TTLExpired,
    AddressNotSupported,
    Unsupported
}

//FIXME: Calling write/read ... functions directly is very inefficient we should use a buffer
//       instead, but we can't just use a BufferedStream in TcpChain node.

#[allow(dead_code)]
impl ReplyType {
    // find if there is a better way of doing this.
    fn from_num(num: u8) -> ReplyType {
        match num {
            0 => { return ReplyType::Succeeded; }
            1 => { return ReplyType::GeneralFailure; }
            2 => { return ReplyType::Unreachable; }
            3 => { return ReplyType::Refused; }
            4 => { return ReplyType::TTLExpired; }
            5 => { return ReplyType::AddressNotSupported; }
            _ => { return ReplyType::Unsupported; }
        }
    }

    fn to_num(&self) -> u8 {
        match &self {
            ReplyType::Succeeded => { return 0; }
            ReplyType::GeneralFailure => { return 1; }
            ReplyType::Unreachable => { return 2; }
            ReplyType::Refused => { return 3; }
            ReplyType::TTLExpired => { return 4; }
            ReplyType::AddressNotSupported => { return 5; }
            ReplyType::Unsupported => { return 6; }
        }
    }
}

impl LinkAddr {
    async fn write(&self, output: &mut (dyn AsyncRW)) -> Result<()> {
        match &self {
            LinkAddr::IPv4(addr) => { 
                output.write_u8(0x01).await?; // Addr type.
                output.write_all(&addr.octets()).await?;
            }
            LinkAddr::IPv6(addr) => {
                output.write_u8(0x04).await?; // Addr type
                output.write_all(&addr.octets()).await?;
            }
            LinkAddr::Domain(domain) => {
                if domain.len() > 0xFF {
                    return Err(Error::new(ErrorKind::InvalidData, "Domain name too long"));
                }
                output.write_u8(0x03).await?; // Addr type
                output.write_u8(domain.len() as u8).await?;
                output.write_all(domain).await?;
            } 
        }
        return Ok(());
    }

    async fn read(read: &mut (dyn AsyncRW)) -> Result<LinkAddr> {
        let addr_type:u8 = read.read_u8().await?;
        match addr_type {
            1 => { // ipv4 addr
               let ip_encoded = read.read_u32().await?;
               return Ok(LinkAddr::IPv4(Ipv4Addr::from(ip_encoded))); 
            }
            3 => { // domain addr.
                let size = read.read_u8().await.unwrap(); 
                let mut buffer = vec![0u8; size.into()];
                read.read_exact(&mut buffer).await?;
                //NOTE: utf8 ?
                return Ok(LinkAddr::Domain(buffer)); 
            }
            // TODO: Implement IPv6  
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Unknown/Unsuported addr type"));
            }
        }
    }
}

#[allow(dead_code)]
struct Reply {
    pub reptype: ReplyType,
    pub addr: LinkAddr,
    pub port: u16
}

// Encode the auth option packet.
#[allow(dead_code)]
async fn send_option(output: &mut (dyn AsyncRW)) -> Result<()> {
    let version: u8 = 0x5;
    output.write_u8(version).await?;
    // We only support no login.
    output.write_u8(1).await?;
    output.write_u8(0).await?;

    output.flush().await?;
    return Ok(());
}

async fn read_options(input: &mut (dyn AsyncRW)) -> Result<Vec<u8>> {
    let count = input.read_u8().await?;
    let mut result = Vec::<u8>::new();
    result.reserve(count as usize);
    input.read_exact(&mut result).await?;
    return Ok(result); 
}

#[allow(dead_code)]
async fn read_auth_response(link: &mut (dyn AsyncRW)) -> Result<()> {
    if link.read_u8().await? != 0x5 {
        return Err(Error::new(ErrorKind::Unsupported, "Unsuported SOCKS Version."));
    } else if link.read_u8().await? != 0 {
        return Err(Error::new(ErrorKind::PermissionDenied, "Auth failed."));
    }

    return Ok(()); 
}   

// TODO: Send UDP Requests too   
#[allow(dead_code)]
async fn send_request(output: &mut (dyn AsyncRW), req: LinkRequest) -> Result<()> {
    let version: u8 = 0x5;
    output.write_u8(version).await?;
    output.write_u8(0x01).await?; // Connect
    output.write_u8(0).await?; // RSV
    req.addr.write(output).await?;

    output.write_u16(req.port).await?;
    output.flush().await?;

    return Ok(()); 
}

async fn read_request(input: &mut (dyn AsyncRW)) -> Result<LinkRequest> {
    if input.read_u8().await? != 0x5 {
        return Err(Error::new(ErrorKind::Unsupported, "Unsuported SOCKS Version."));
    } 
    let req_type = input.read_u8().await?; 
    if req_type != 1 {
        return Err(Error::new(ErrorKind::Unsupported, "Only CONNECT request type is supported."));
    }
    input.read_u8().await?; // RSV.

    let link_addr = LinkAddr::read(input).await?;
    let port = input.read_u16().await?;
    return Ok(LinkRequest { addr: link_addr, port: port});
}

async fn read_response(input: &mut (dyn AsyncRW)) -> Result<Reply> {
    if input.read_u8().await? != 0x5 {
        return Err(Error::new(ErrorKind::InvalidData, "Incorrect protocol version"));
    }
    let reply = ReplyType::from_num(input.read_u8().await?);
    println!("reply type: {:?}", reply);
    input.read_u8().await?;
    let addr = LinkAddr::read(input).await?; 
    let port = input.read_u16().await?; 

    return Ok(Reply{ reptype: reply, addr: addr, port: port});
} 

/*
async fn write_response(output: &mut (dyn AsyncRW)) -> Result<Reply> {
    output.read_u8().await? != 0x5 {
    let reply = ReplyType::to_num(input.read_u8().await?);
    println!("reply type: {:?}", reply);
    let rsv = input.read_u8().await?;
    let addr = LinkAddr::read(input).await?; 
    let mut port = input.read_u16().await?; 

    return Ok(Reply{ reptype: reply, addr: addr, port: port});
} 
*/

#[allow(dead_code)]
async fn connect_socks5(link:&mut (dyn AsyncRW), req: LinkRequest) -> Result<()> {
    println!("sending auth options");
    send_option(link).await?;
    println!("reading auth response");
    read_auth_response(link).await?;
    println!("sending request"); 
    send_request(link, req).await?;
    println!("reading response");
    read_response(link).await?;

    return Ok(());
}

pub struct Socks5ChainNode {
    pub proxy_addr: SocketAddr, 
    pub parent: Box<dyn ChainNode>
}

#[async_trait]
impl ChainNode for Socks5ChainNode {
    async fn connect(&self, req: LinkRequest) -> Result<Box<dyn AsyncRW>> {
        // First connect to the proxy server through our parent node.
        let proxy_request = LinkRequest::from(self.proxy_addr);
        let mut stream = self.parent.connect(proxy_request).await?;
        connect_socks5(&mut stream, req).await?;
        return Ok(stream); 
    }
}

// --- Server Code ---
async fn socks5_handle_connection(socket: &mut TcpStream) -> Result<()> {

    // Read auth options.
    read_options(socket).await?;

    //FIXME: Better handling of auth.

    // Send auth response.
    send_option(socket).await?;

    // Read request.
    let _link_req = read_request(socket).await?;
    
    // Connect to the address through the chain.
        
    // Write response. 

    return Ok(());
}

async fn socks5_server(config: Yaml) {
    let address : SocketAddr = config["addr"].as_str().unwrap().parse().unwrap();
    let listener = TcpListener::bind(address).await.unwrap(); 
    println!("[SOCKS5 Server] Listening"); 

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        println!("[SOCKS5 Server] Accepted a connection");
        tokio::spawn(async move {
            socks5_handle_connection(&mut socket).await;
        });
    }
}

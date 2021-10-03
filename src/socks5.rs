use async_trait::async_trait;

use yaml_rust::Yaml;

use tokio::io::*;
use tokio::net::*;
use std::net::{Ipv4Addr, SocketAddr};

use std::io::Result;
use std::sync::Arc;
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

//NOTE:  Maybe we can use a buffered stream on read side only and encode the packets to vectors
//       before sending them. 


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
    async fn write(&self, output: &mut (dyn AsyncW)) -> Result<()> {
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

    async fn read(read: &mut (dyn AsyncR)) -> Result<LinkAddr> {
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
                return Err(Error::new(ErrorKind::InvalidData, "Unknown/Unsupported addr type"));
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
// Client only
#[allow(dead_code)]
async fn send_option(output: &mut (dyn AsyncW)) -> Result<()> {
    let version: u8 = 0x5;
    output.write_u8(version).await?;
    // We only support no login.
    output.write_u8(1).await?;
    output.write_u8(0).await?;

    output.flush().await?;
    return Ok(());
}

// Server only
async fn read_options(input: &mut (dyn AsyncR)) -> Result<Vec<u8>> {
    let version = input.read_u8().await?; 
    if version != 0x5 {
        println!("Incorrect version: {:?}", version);
        return Err(Error::new(ErrorKind::Unsupported, "Unsupported SOCKS Version."));
    } 



    let count = input.read_u8().await?;
    println!("Count: {:?}", count);
    let mut options = vec![0u8; count as usize];
    input.read_exact(&mut options).await?;
    println!("Options vector: {:?}", options);

    return Ok(options);
}

// Client only.
#[allow(dead_code)]
async fn read_auth_response(link: &mut (dyn AsyncR)) -> Result<()> {
    let version = link.read_u8().await?; 
    if version != 0x5 {
        println!("Incorrect version: {:?}", version);
        return Err(Error::new(ErrorKind::Unsupported, "Unsupported SOCKS Version."));
    } else if link.read_u8().await? != 0 {
        return Err(Error::new(ErrorKind::PermissionDenied, "Auth failed."));
    }

    return Ok(()); 
}   


// Server only
async fn send_auth_response(output: &mut (dyn AsyncW)) -> Result<()> {
    output.write_u8(0x5).await?;
    output.write_u8(0x0).await?;
    return Ok(());
} 

// TODO: Send UDP Requests too   
// Client only.
#[allow(dead_code)]
async fn send_request(output: &mut (dyn AsyncW), req: LinkRequest) -> Result<()> {
    let version: u8 = 0x5;
    output.write_u8(version).await?;
    output.write_u8(0x01).await?; // Connect
    output.write_u8(0).await?; // RSV
    req.addr.write(output).await?;

    output.write_u16(req.port).await?;
    output.flush().await?;

    return Ok(()); 
}

// Server only.
async fn read_request(input: &mut (dyn AsyncR)) -> Result<LinkRequest> {
    let version = input.read_u8().await?; 
    if version != 0x5 {
        println!("REQ Incorrect protocol version {:?}", version);
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

//Client only
async fn read_response(input: &mut (dyn AsyncR)) -> Result<Reply> {
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

//Server only
async fn send_response(input: &mut (dyn AsyncW), reptype: ReplyType) -> Result<()> {
    input.write_u8(0x5).await?;
    input.write_u8(ReplyType::to_num(&reptype)).await?;
    input.write_u8(0).await?;
    
    // Empty ip address. 
    input.write_u8(1).await?;
    input.write_u32(0).await?;
    input.write_u16(0).await?;
    
    return Ok(()); 
} 

#[allow(dead_code)]
async fn connect_socks5(link:&mut StreamPair, req: LinkRequest) -> Result<()> {
    println!("sending auth options");
    send_option(&mut link.write).await?;
    println!("reading auth response");
    read_auth_response(&mut link.read).await?;
    println!("sending request"); 
    send_request(&mut link.write, req).await?;
    println!("reading response");
    read_response(&mut link.read).await?;

    return Ok(());
}

#[derive(Debug)]
pub struct Socks5ChainNode {
    pub proxy_addr: SocketAddr, 
    pub parent: Arc<dyn ChainNode>
}

#[async_trait]
impl ChainNode for Socks5ChainNode {
    async fn connect(&self, req: LinkRequest) -> Result<StreamPair> {
        // First connect to the proxy server through our parent node.
        let proxy_request = LinkRequest::from(self.proxy_addr);
        let mut stream = self.parent.connect(proxy_request).await?;
        // Create a temporary BufStream. 
        connect_socks5(&mut stream, req).await?;
        return Ok(stream); 
    }
}

//NOTE: Maybe consider having a some sort of connection pool system ?
struct Socks5ServerState {
    pub chain: Arc<dyn ChainNode>
}

// --- Server Code ---
async fn socks5_handle_connection(mut socket: TcpStream, server_state: &mut Arc<Socks5ServerState> ) -> Result<()> {
    // - Read auth options -
    read_options(&mut socket).await?;
    // - Send auth response - 
    send_auth_response(&mut socket).await?;

    // - Read request -
    let link_req = read_request(&mut socket).await?;
    
    // - Connect to the address through the chain. -
    //NOTE: Failure to connect is not a protocol/client error.
    // we should send a response if we fail. 
    let con_result = server_state.chain.connect(link_req).await; 
    let response_type = match con_result {
        Ok(_) => ReplyType::Succeeded,
        Err(_) => ReplyType::GeneralFailure
    };

    if con_result.is_err() {
        println!("Connection request to chain failed.");
    }

    // Write response. 
    send_response(&mut socket, response_type).await?;
    
    if con_result.is_err() {
        // We sent the error response, so this is ok.
        return Ok(());
    } 
    let mut con = con_result.unwrap();

    let (mut sock_read, mut sock_write) = socket.split();    

    // Start copying in both directions.
    let (up_copy, down_copy) = tokio::join!(
            copy(&mut sock_read, &mut con.write),
             copy(&mut con.read, &mut sock_write));

    // Handle errors.
    up_copy?;
    down_copy?;

    return Ok(());
}

pub async fn socks5_server(config: &Yaml) {
    println!("Initializing socks5 server");


    let address : SocketAddr = config["addr"].as_str().unwrap().parse().unwrap();
    let chain_config  = config["nodes"].as_vec().unwrap();
    let chain = load_chain(chain_config);
    println!("Chain loading complete");


    let listener = TcpListener::bind(address).await.unwrap(); 
    println!("[SOCKS5 Server] Listening"); 

    tokio::spawn(async move {
        let server_state = Arc::new(Socks5ServerState { chain: chain });
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let mut handle = server_state.clone();
            println!("[SOCKS5 Server] Accepted a connection");
            tokio::spawn(async move {
                let result = socks5_handle_connection(socket, &mut handle).await;

                println!("Connection Terminated {:?}", result); 

            });
        }
    }).await;
}

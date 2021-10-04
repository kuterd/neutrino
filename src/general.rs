use async_trait::async_trait;

use tokio::io::*;
use tokio::net::*;
use std::net::{Ipv4Addr, Ipv6Addr, IpAddr, SocketAddr};
use std::io::Result;
use std::fmt::Debug;
use std::sync::Arc;
use yaml_rust::yaml::*;

use crate::socks5::*;


// Connection result.
#[derive(Debug)]
pub enum ReplyType {
    Succeeded,
    GeneralFailure,
    Unreachable,
    Refused,
    TTLExpired,
    AddressNotSupported,
    Unsupported
}

pub trait AsyncW: AsyncWrite + Unpin + Send {}
impl<T> AsyncW for T where T: AsyncWrite + Unpin + Send {}

pub trait AsyncR: AsyncRead + Unpin + Send {}
impl<T> AsyncR for T where T: AsyncRead + Unpin + Send {}

pub trait AsyncRW: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> AsyncRW for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

// We need this because we can't combine two unrelated streams together.
pub struct StreamPair {
    pub read: Box<dyn AsyncR>,
    pub write: Box<dyn AsyncW>
}

// Address of a connection
// std::net::SocketAddr can not represent a domain address.
#[allow(dead_code)]
#[derive(Debug)]
pub enum LinkAddr {
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
    Domain(Vec<u8>),
    Unknown()
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
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Unknown address "));
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


pub trait GeneralChainNode: InputChainNode + OutputChainNode {}
impl<T> GeneralChainNode for T where T: InputChainNode + OutputChainNode {}

// General chain node.
#[derive(Debug)]
pub enum ChainNode {
    // A general chain node can be both be a input and a output.
    // It can behave differently in different roles.
    //General(Arc<dyn GeneralChainNode>),
    Input(Arc<dyn InputChainNode>),
    Output(Arc<dyn OutputChainNode>)
}

// Standart information about a chain node.
#[derive(Debug)]
pub struct ChainNodeInfo {
    pub name: Option<String>,
    pub parent: Option<ChainNode>
}

// A OutputChainNode opens a connection for a given link request.
#[async_trait]
pub trait OutputChainNode: Send + Sync + Debug {
    // Open a connection for the given link request.
    async fn connect(&self, req: LinkRequest) -> Result<StreamPair>;
}

// Input chain node handles a incoming connection (used in server side)
// it translates the incoming conncetion to a connection requst.
#[async_trait]
pub trait InputChainNode: Send + Sync + Debug {
    async fn handle_connection(&self, stream: &mut StreamPair) -> Result<LinkRequest>;
    async fn send_response(&self, stream: &mut StreamPair, reply: ReplyType) -> Result<()>;
}


fn load_input_chain(chain: &Array) -> Arc<dyn InputChainNode> {
    println!("[Input Chain Loader] Loading input chain");
    let mut parent : Option<Arc<dyn InputChainNode>> = None;
    for node in chain {
        let mut name : Option<String> = None;
        let mname = node["name"].as_str();
        if let Some(st) = mname {
            name = Some(st.to_string());
        }

        println!("[Input Chain Loader] Node name: {:?}", name);
        let mut parent_wrapped: Option<ChainNode> = None;
        if let Some(sparent) = parent.clone() {
            parent_wrapped = Some(ChainNode::Input(sparent));
        }

        let node_info = ChainNodeInfo { name: name,  parent: parent_wrapped};
        let n_type = node["type"].as_str()
            .expect("expected a 'type' for the chain node");
        println!("[Input Chain Loader] Node type: {}", n_type);

        match n_type {
            "socks5_server" => {
                parent =
                     Some(Arc::new(Socks5InputChainNode {}));
            }
            "header_injector" => {
                let inj = Vec::from(node["injection"].as_str().unwrap());
                parent = Some(Arc::new(HeaderInjectorChainNode { node_info: node_info, injection: inj}));
            }
            _ => {
                panic!("Unknown node");
            }
        }
    }
    return parent.unwrap();
}

// Constructs a chain from a Yaml array.
fn load_output_chain(chain: &Array) -> Arc<dyn OutputChainNode> {
    println!("[Output Chain Loader] Loading output chain");
    let mut parent : Option<Arc<dyn OutputChainNode>> = None;
    for node in chain {
        let mut name : Option<String> = None;
        let mname = node["name"].as_str();
        if let Some(st) = mname {
            name = Some(st.to_string());
        }

        println!("name: {:?}", name);

        let mut parent_wrapped: Option<ChainNode> = None;
        if let Some(sparent) = parent.clone() {
            parent_wrapped = Some(ChainNode::Output(sparent));
        }

        let node_info = ChainNodeInfo { name: name, parent: parent_wrapped};
        let n_type = node["type"].as_str()
            .expect("expected a 'type' for the chain node");
        println!("[Output Chain Loader] Node type: {}", n_type);

        match n_type {
            "socks5" => {
                assert!(parent.is_some(), "socks5 node must not be the first node in the chain!");
                let addr = node["addr"].as_str().unwrap().parse().unwrap();
                parent =
                     Some(Arc::new(Socks5OutputChainNode {parent: parent.unwrap(), proxy_addr: addr}));
            }
            "header_injector" => {
                assert!(parent.is_some(), "header_injector node must not be the first node in the chain!");
                let inj = Vec::from(node["injection"].as_str().unwrap());
                parent = Some(Arc::new(HeaderInjectorChainNode { node_info: node_info, injection: inj}));
            }
            "tcp" => {
                assert!(parent.is_none(), "tcp node must be the first node in the chain!");
                parent = Some(Arc::new(TcpOutputChainNode {node_info: node_info}));
            }
            _ => {
                panic!("Unknown node");
            }
        }
    }
    return parent.unwrap();
}

#[derive(Clone)]
pub struct ChainSet {
    output_chain: Arc<dyn OutputChainNode>,
    input_chain: Arc<dyn InputChainNode>,
    address: SocketAddr
}

impl ChainSet {
    pub async fn load(chain_config: &Yaml) -> ChainSet {
        println!("Config {:?}", chain_config);

        let input_chain = load_input_chain(&chain_config["input"].as_vec()
            .expect("expected 'input' to be a array"));
        let output_chain = load_output_chain(&chain_config["output"].as_vec()
            .expect("expected 'output' to be a array"));
        let addr = chain_config["addr"].as_str().unwrap().parse().unwrap();

        return ChainSet { input_chain: input_chain, output_chain: output_chain, address: addr};
    }

    async fn run_internal(&self) -> Result<()> {
        let listener = TcpListener::bind(self.address).await.unwrap();
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let input_chain = self.input_chain.clone();
            let output_chain = self.output_chain.clone();
            println!("[TCP Server] Accepted a connection");
            tokio::spawn(async move {
                let (read, write) = socket.into_split();
                let mut stream = StreamPair {read: Box::new(read), write: Box::new(write)};
                println!("[TCP Server] Routing connection through input chain");
                let request_res = input_chain.handle_connection(&mut stream).await;
                if let Err(err) = request_res {
                    println!("[TCP Server] Input chain error: {:?}", err);
                    println!("[TCP Server] Terminating connection");
                    return;
                }
                let request = request_res.unwrap();
                println!("[TCP Server] Request resolved: {:?}", request);
                println!("[TCP Server] Routing request through output chain");
                let con_res = output_chain.connect(request).await;

                if let Err(err) = con_res {
                    println!("[TCP Server] Output chain connection error: {:?}", err);
                    //FIXME: More detailed error reporting to the client.
                    let _ = input_chain.send_response(&mut stream, ReplyType::GeneralFailure).await;
                    return;
                }
                println!("[TCP Server] Sending success response");
                let _ = input_chain.send_response(&mut stream, ReplyType::Succeeded).await;

                let mut con = con_res.unwrap();
                println!("[TCP Server] Starting to copy in both directions");

                // Start copying in both directions.
                let (_up_copy, _down_copy) = tokio::join!(
                    copy(&mut stream.read, &mut con.write),
                     copy(&mut con.read, &mut stream.write));
                return;
            });
        }
    }

    pub async fn run(&self) -> Result<()> {
        let copy = self.clone();
        tokio::spawn(async move {
            //FIXME: Error here shouldn't crash the server.
            return copy.run_internal().await;
        }).await?;
        return Ok(());
    }
}

#[derive(Debug)]
pub struct TcpOutputChainNode {
    pub node_info: ChainNodeInfo
}

#[async_trait]
impl OutputChainNode for TcpOutputChainNode {
//       Sadly, we can't use a buffered stream here.
//       I will eventually fix this limitation.
//       return Ok(Box::<BufStream<TcpStream>>::new(BufStream::new(stream)));
    async fn connect(&self, req: LinkRequest) -> Result<StreamPair> {
       println!("[TCP Output Node] Opening a connection for request: {:?}", req);
       let stream = Box::new(TcpStream::connect(req.as_socket_addr().await?).await?);
       println!("[TCP Output Node] Connection open");
       let (read, write) = stream.into_split();
       return Ok(StreamPair {read: Box::new(read), write: Box::new(write)});
    }
}

#[derive(Debug)]
pub struct HeaderInjectorChainNode {
    pub node_info: ChainNodeInfo,
    pub injection: Vec<u8>
}

#[async_trait]
impl OutputChainNode for HeaderInjectorChainNode {
    async fn connect(&self, req: LinkRequest) -> Result<StreamPair> {
        match self.node_info.parent.as_ref().unwrap() {
            ChainNode::Output(output) => {
                let mut stream = output.connect(req).await?;
                println!("[Header Injector] Injecting the header string");
                stream.write.write_all(&self.injection).await?;
                return Ok(stream);
            }
            _ => {
                unreachable!("Expected output chain for Header injection");
            }
        }
    }
}

#[async_trait]
impl InputChainNode for HeaderInjectorChainNode {
    async fn handle_connection(&self, stream: &mut StreamPair) -> Result<LinkRequest> {
         match self.node_info.parent.as_ref().unwrap() {
            ChainNode::Input(input) => {
                let mut buffer = vec![0u8; self.injection.len()];
                stream.read.read_exact(&mut buffer).await?;
                println!("[Header Injector] Stripped away {} bytes of junk.", self.injection.len());
                return input.handle_connection(stream).await;
            }
            _ => {
                unreachable!("Expected output chain for Header injection");
            }
         }
    }
    async fn send_response(&self, stream: &mut StreamPair, reply: ReplyType) -> Result<()> {
       match self.node_info.parent.as_ref().unwrap() {
            ChainNode::Input(input) => {
                println!("[Header injector] asking parent to send response");
                return input.send_response(stream, reply).await;
            }
            _ => {
                unreachable!("Expected output chain for Header injection");
            }
        }
    }
}


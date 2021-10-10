
![Neutrino Logo](https://github.com/kuterd/neutrino/blob/master/branding/neutrino-logo-small.png?raw=true)


# Neutrino: Modular proxy 

Neutrino is a modular proxy. It is inspired by ideas like Proxy Chaining, Shadowsocks, HTTP Injection and Pluggable Transports.
Neutrino modules can be arranged to serve different purposes.
Neutrino can be configured to work in networks that block proxy protocols by injecting a fake protocol header and encrypting the proxy communication.

A Neutrino configuration has two parts a `Input` chain and a `Output` chain.  
Neutrino acts as a server and routes the incoming connection through the `Input` chain, after the request is resolved a request is made through the `Output` chain.

The `Output` chain might simply be a single node that opens a socket and routes the traffic to the internet.
Or it may be a node that routes it to a SOCKS5 proxy. You can use the `header_injection` node in the `Output` chain to inject a header to the communication stream, this can make a firewall think this is a HTTP connection
rather than a proxy connection. 
Please note that doing this breaks compatibility with the regular SOCKS5 protocol.
You need another Neutrino instance running in a trusted server that has a `header_injection` in its `Input` chain
to strip away the fake header before the SOCKS5 handler node. 

## Typical configurations

### HTTP Header Injection for censorship bypass

Neutrino can inject a fake HTTP header before the SOCKS5 proxy communication starts.
This can make it possible to use proxy in a network that disallows proxy communication by hiding the protocol.
This breaks the compatibility with the SOCKS5 protocol standard but Neutrino can also act as a server
and strip away the fake HTTP header in it's input chain. 
Here is a diagram of how this works:
![Neutrino Protocol Header injection diagram](https://github.com/kuterd/neutrino/blob/master/branding/neutrino_injection_explained.png?raw=true)
Note: TLS Support is a WIP.

### Proxy on top of Tor

Most Tor exit nodes are flagged by site protection systems. Most sites might require you to fill captcha or 
even make it impossible to use a site. So it might be desirable to hide the fact that you are using Tor.
This is a simple configuration that allows that.

### Proxy chaining

![Neutrino proxy chaining diagram](https://github.com/kuterd/neutrino/blob/master/branding/neutrino_chain_explained.png?raw=true)

You can have multiple `socks5` nodes in your input chain to route your traffic through multiple proxy servers. 
Note: Proxy chaining is not very safe. Since the SOCKS5 protocol doesn't have any encryption the first proxy server in your chain can read your traffic.    

### Socks5 with TLS (WIP)


 

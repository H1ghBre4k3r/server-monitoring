use std::net::Ipv4Addr;

const AGENT_PORT: &str = "AGENT_PORT";

const DEFAULT_PORT: u16 = 51243;

pub fn get_default_port() -> u16 {
    DEFAULT_PORT
}

pub fn get_port() -> u16 {
    let port_from_env = std::env::var(AGENT_PORT);
    port_from_env.map_or(DEFAULT_PORT, |res| res.parse().unwrap_or(DEFAULT_PORT))
}

const AGENT_ADDR: &str = "AGENT_ADDR";

const DEFAULT_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

pub fn get_addr() -> Ipv4Addr {
    let addr_from_env = std::env::var(AGENT_ADDR);
    addr_from_env.map_or(DEFAULT_ADDR, |res| res.parse().unwrap_or(DEFAULT_ADDR))
}

const AGENT_SECRET: &str = "AGENT_SECRET";

pub fn get_secret() -> Option<String> {
    let secret_from_env = std::env::var(AGENT_SECRET);
    secret_from_env.ok()
}

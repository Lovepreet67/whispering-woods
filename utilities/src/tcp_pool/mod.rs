use crate::result::Result;

#[derive(Debug, Default)]
pub struct TcpPool {}
impl TcpPool {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(&self, tcp_address: &str) -> Result<tokio::net::TcpStream> {
        tokio::net::TcpStream::connect(tcp_address)
            .await
            .map_err(|e| {
                format!("Error while connecting to stream at {tcp_address:?} {e:?}").into()
            })
    }
}

pub static TCP_CONNECTION_POOL: once_cell::sync::Lazy<TcpPool> =
    once_cell::sync::Lazy::new(TcpPool::new);

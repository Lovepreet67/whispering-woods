use std::error::Error;

#[derive(Debug, Default)]
pub struct TcpPool {}
impl TcpPool {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(
        &self,
        tcp_address: &str,
    ) -> Result<tokio::net::TcpStream, Box<dyn Error>> {
        tokio::net::TcpStream::connect(tcp_address)
            .await
            .map_err(|e| {
                format!("Error while connecting to stream at {tcp_address:?} {e:?}").into()
            })
    }
}

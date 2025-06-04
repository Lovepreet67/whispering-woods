use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream, duplex},
    net::TcpStream,
};

pub fn tee_tcp_stream(mut tcp_stream: TcpStream) -> (DuplexStream, DuplexStream) {
    let (mut tx1, rx1) = duplex(8192); // 8192 is 8kb
    let (mut tx2, rx2) = duplex(8192);
    tokio::spawn(async move {
        let mut buf = [0u8; 8192];
        loop {
            let n = tcp_stream.read(&mut buf).await.unwrap_or(0);
            if n == 0 {
                break;
            }
            tx1.write_all(&buf[0..n]).await.unwrap();
            tx2.write_all(&buf[0..n]).await.unwrap();
        }
        drop(tx1);
        drop(tx2);
    });
    (rx1, rx2)
}

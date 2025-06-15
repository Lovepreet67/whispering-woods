use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream, duplex},
    net::TcpStream,
};
use utilities::logger::{Instrument, Span, debug, trace};

pub fn tee_tcp_stream(mut tcp_stream: TcpStream) -> (DuplexStream, DuplexStream) {
    let span = Span::current();
    let (mut tx1, rx1) = duplex(8192); // 8192 is 8kb
    let (mut tx2, rx2) = duplex(8192);
    tokio::spawn(
        async move {
            let mut buf = [0u8; 8192];
            let mut x = 0;
            loop {
                trace!("Got {x} block of data from tcp stream");
                x += 1;
                let n = tcp_stream.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    debug!("breaking from the loop");
                    break;
                }
                tx1.write_all(&buf[0..n]).await.unwrap();
                tx2.write_all(&buf[0..n]).await.unwrap();
            }
            drop(tx1);
            drop(tx2);
        }
        .instrument(span),
    );
    (rx1, rx2)
}

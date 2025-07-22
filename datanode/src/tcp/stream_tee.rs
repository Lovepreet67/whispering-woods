use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream, duplex},
    net::TcpStream,
};
use utilities::logger::{Instrument, Span, error, trace};

pub fn tee_tcp_stream(mut tcp_stream: TcpStream) -> (DuplexStream, DuplexStream) {
    let span = Span::current();
    let (mut tx1, rx1) = duplex(8192); // 8192 is 8kb
    let (mut tx2, rx2) = duplex(8192);
    tokio::spawn(
        async move {
            let mut buf = [0u8; 8192];
            let mut x = 0;
            let mut total_read = 0;
            loop {
                trace!("Got {x} block of data from tcp stream");
                x += 1;
                let n = tcp_stream.read(&mut buf).await.unwrap_or(0);
                total_read += n;
                if n == 0 {
                    if let Err(e) = tcp_stream.write_u64(total_read as u64).await {
                        error!(error=%e,"Error while writing byte count to tcp stream")
                    }
                    break;
                }
                tx1.write_all(&buf[0..n]).await.unwrap();
                tx2.write_all(&buf[0..n]).await.unwrap();
            }
            tx1.shutdown().await;
            tx2.shutdown().await;
            // now will check the bytes received by both streams
            trace!("all bytes written waiting for bytes written count");
            let bytes_written_to_pipeline = tx1.read_u64().await.unwrap_or(0);
            let bytes_written_to_file = tx2.read_u64().await.unwrap_or(0);
            if bytes_written_to_file!=bytes_written_to_pipeline {
                error!("Error diffrent number of bytes written to pipeline({bytes_written_to_pipeline}) and bytes written to file ({bytes_written_to_pipeline})");
            }
            tcp_stream.write_u64(std::cmp::min(bytes_written_to_pipeline, bytes_written_to_file)).await;
            drop(tx1);
            drop(tx2);
        }
        .instrument(span),
    );
    (rx1, rx2)
}

RUST_LOG=trace cargo run -p namenode -- 7000
RUST_LOG=trace cargo run -p datanode -- 9000 4000  
RUST_LOG=trace cargo run -p client -- http://127.0.0.1:7000 


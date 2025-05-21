mod chunk_handler;
mod file_handler;
mod namenode_handler;
fn main() {
    let namenode_addrs = std::env::args()
        .nth(1)
        .expect("Please provide Name Node address.");
}

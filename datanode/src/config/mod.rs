use figment::{providers::{Format, Yaml}, Figment};
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug,Deserialize)]
pub struct Config{
    pub datanode_id: String,
    pub namenode_addrs:String,
    pub internal_grpc_port:String,
    pub internal_tcp_port:String,
    pub external_grpc_addrs:String,
    pub external_tcp_addrs: String,
    pub storage_path: String,
    pub log_level:String
}

pub static CONFIG: Lazy<Config> = Lazy::new(||{
    let env = std::env::var("ENV").unwrap_or_else(|_|"development".to_owned());
    let config_file_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| format!("./config/{}.yaml",env));
    println!("Reading config from file : {config_file_path}");
    Figment::new().merge(Yaml::file(config_file_path)).extract().unwrap()
}); 

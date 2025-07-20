use figment::{
    Figment,
    providers::{self, Format},
};
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub client_id: String,
    pub namenode_addrs: String,
    pub log_level: String
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let env = std::env::var("ENV").unwrap_or_else(|_|"development".to_owned());
    // giving defaule path to root of binary
    let config_file_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| format!("./config/{}.yaml",env));
    println!("reading config from {config_file_path:?}");
    Figment::new()
        .merge(providers::Yaml::file(config_file_path))
        .extract()
        .unwrap()
});

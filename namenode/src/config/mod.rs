use figment::{
    Figment,
    providers::{Format, Yaml},
};
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub id: String,
    pub internal_grpc_port: String,
    pub external_grpc_addrs: String,
    pub ledger_file: String,
    pub log_level: String,
    pub log_base: String,
    pub state_log_file: Option<String>,
    pub apm_endpoint: String,
}
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let env = std::env::var("ENV").unwrap_or_else(|_| "default".to_owned());
    let config_file_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| format!("./namenode/config/{}.yaml", env));
    Figment::new()
        .merge(Yaml::file(config_file_path))
        .extract()
        .unwrap()
});

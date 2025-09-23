use figment::{
    Figment,
    providers::{Format, Serialized, Yaml},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

fn default_api_username() -> String {
    "username".to_string()
}
fn default_api_password() -> String {
    "password".to_string()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub id: String,
    pub internal_grpc_port: String,
    pub external_grpc_addrs: String,
    pub ledger_file: String,
    pub log_level: String,
    pub log_base: String,
    pub state_log_file: Option<String>,
    pub apm_endpoint: String,
    pub api_port: Option<u16>,

    #[serde(default = "default_api_username")]
    pub api_username: String,
    #[serde(default = "default_api_password")]
    pub api_password: String,
    pub api_jwt_sign_key: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            id: "id".to_string(),
            internal_grpc_port: 7000.to_string(),
            external_grpc_addrs: "http://127.0.0.1:7000".to_string(),
            ledger_file: "./temp/namenode/history.log".to_string(),
            log_level: "trace".to_string(),
            log_base: "./temp/namenode/".to_string(),
            state_log_file: Some("./temp/namenode/state.log".to_string()),
            apm_endpoint: "http://127.0.0.1:8200/v1/traces".to_string(),
            api_port: Some(8080),
            api_username: "username".to_string(),
            api_password: "password".to_string(),
            api_jwt_sign_key: "key".to_string(),
        }
    }
}
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let env = std::env::var("ENV").unwrap_or_else(|_| "default".to_owned());
    let config_file_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| format!("./namenode/config/{}.yaml", env));
    Figment::new()
        .merge(Serialized::default("default", Config::default()))
        .merge(Yaml::file(config_file_path))
        .extract()
        .unwrap()
});

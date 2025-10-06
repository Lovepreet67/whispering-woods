use figment::{
    Figment,
    providers::{Format, Yaml},
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use storage::file_storage::FileStorageConfig;

fn default_false() -> bool {
    false
}
#[derive(Clone, Debug, Deserialize)]
pub struct StorageConfig {
    // path to the dir where data will be stored
    pub storage_path: String,
    #[serde(default = "default_false")]
    // this will only be checked in case the storage path provided doesn't exist if it exist we
    // will use that path as it is assuming that node is restarting or thats what user intended
    pub create_mount: bool,
    #[serde(default)]
    // thi will be used in case we want to create a mount
    pub mount_size_in_mega_byte: u64,
}
impl Into<FileStorageConfig> for StorageConfig {
    fn into(self) -> FileStorageConfig {
        FileStorageConfig {
            root: self.storage_path,
            create_mount: self.create_mount,
            mount_size_in_mega_byte: self.mount_size_in_mega_byte,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub datanode_id: String,
    pub namenode_addrs: String,
    pub internal_grpc_port: String,
    pub internal_tcp_port: String,
    pub external_grpc_addrs: String,
    pub external_tcp_addrs: String,
    pub storage_config: StorageConfig,
    pub log_level: String,
    pub log_base: String,
    pub apm_endpoint: String,
    pub namenode_cert: String,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let env = std::env::var("ENV").unwrap_or_else(|_| "default".to_owned());
    let config_file_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| format!("./datanode/config/{}.yaml", env));
    println!("Reading config from file : {config_file_path}");
    Figment::new()
        .merge(Yaml::file(config_file_path))
        .extract()
        .unwrap()
});

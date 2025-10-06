use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeType {
    Unknown,
    Datanode,
    Namenode,
    Client,
}
impl From<Option<&&str>> for NodeType {
    fn from(value: Option<&&str>) -> Self {
        if let Some(val) = value {
            match *val {
                "Client" => NodeType::Client,
                "Datanode" => NodeType::Datanode,
                "Namenode" => NodeType::Namenode,
                _ => NodeType::Unknown,
            }
        } else {
            NodeType::Unknown
        }
    }
}

impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "client" => NodeType::Client,
            "datanode" => NodeType::Datanode,
            "namenode" => NodeType::Namenode,
            _ => NodeType::Unknown,
        }
    }
}
impl Into<String> for NodeType {
    fn into(self) -> String {
        match self {
            Self::Unknown => "Unknown".to_string(),
            Self::Client => "Client".to_string(),
            Self::Datanode => "Datanode".to_string(),
            Self::Namenode => "Namenode".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthType {
    NoAuth,
    JwtTokenAuth,
    CertAuth,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub id: String,
    pub node_type: NodeType,
    pub authenticated_using: AuthType,
}

pub(crate) enum Credentials {
    NoAuthAuth { id: String, node_type: NodeType },
    JwtTokenAuth { token: String },
    CertAuth { cert_pem: String },
}

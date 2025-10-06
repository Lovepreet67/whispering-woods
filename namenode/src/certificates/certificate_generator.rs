use crate::config::CONFIG;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, IsCa, Issuer, KeyPair, PKCS_ECDSA_P256_SHA256,
};
use std::{fs, path::PathBuf};
use utilities::{
    auth::types::NodeType,
    logger::{info, instrument, tracing},
    result::Result,
};

#[derive(Debug)]
pub struct CertificateAuthority {
    issuer: Issuer<'static, KeyPair>,
    cert: Certificate,
}

impl CertificateAuthority {
    #[instrument]
    pub fn new() -> Result<Self> {
        let certificate_dir = PathBuf::from(&CONFIG.certificate_dir);
        if !certificate_dir.exists() {
            info!("Creating certificate dir");
            fs::create_dir_all(&certificate_dir).unwrap();
        }
        let key_file_path = certificate_dir.join("root_key.pem");
        let key_pair = match fs::read_to_string(&key_file_path) {
            Ok(v) => KeyPair::from_pem(&v)?,
            //.map_err("Error while creating a key pair from existing root key".into())?,
            Err(_e) => {
                info!("Error while reading the key file content");
                let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
                    .expect("Error while creating a key pair");
                fs::write(key_file_path, key_pair.serialize_pem())?;
                key_pair
            }
        };
        info!("Creating new root certificates");
        let mut params = CertificateParams::new(vec!["NamenodeRoot".to_string()])?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(rcgen::DnType::OrganizationName, "Namenode");
        let cert = params.self_signed(&key_pair)?;
        let issuer = Issuer::new(params, key_pair);
        Ok(Self { issuer, cert })
    }

    pub fn issue_certificate(&self, node_id: String, node_type: NodeType) -> Result<Certificate> {
        let leaf_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .expect("Error while generating the key pair");

        let mut params = CertificateParams::new(vec!["Datanode".to_string()]).unwrap();

        params.is_ca = IsCa::NoCa;

        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, node_id);
        params
            .distinguished_name
            .push(rcgen::DnType::OrganizationalUnitName, node_type);
        let cert = params.signed_by(&leaf_key, &self.issuer)?;
        Ok(cert)
    }
    // pub fn revoke_certifcate() {}
    pub fn get_root_cert(&self) -> Certificate {
        self.cert.clone()
    }
}

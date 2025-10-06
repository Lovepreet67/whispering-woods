use crate::auth::{
    Authenticator,
    authentication_error::AuthenticationError,
    types::{AuthType, Credentials, NodeMetadata, NodeType},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use pem::Pem;
use std::time::{SystemTime, UNIX_EPOCH};
use webpki::{EndEntityCert, SignatureAlgorithm, Time, TlsClientTrustAnchors, TrustAnchor};
use x509_parser::prelude::{FromDer, X509Certificate};

const SUPPORTED_SIG_ALGS: &[&SignatureAlgorithm] = &[&webpki::ECDSA_P256_SHA256];

#[derive(Clone, Debug)]
pub(crate) struct CertAuthenticator {
    root_cert: Pem,
}

impl CertAuthenticator {
    pub fn new(root_cert_pem: &str) -> Result<Self, AuthenticationError> {
        let cert_parsed = pem::parse(root_cert_pem)
            .map_err(|_| AuthenticationError::Internal("Failed to parse root PEM".to_string()))?;
        Ok(Self {
            root_cert: cert_parsed,
        })
    }
}

impl Authenticator for CertAuthenticator {
    fn authenticate(&self, credentials: Credentials) -> Result<NodeMetadata, AuthenticationError> {
        if let Credentials::CertAuth { cert_pem } = credentials {
            let cert_der = BASE64_STANDARD
                .decode(cert_pem)
                .map_err(|_| AuthenticationError::InvalidCredentials)?;
            let cert = EndEntityCert::try_from(&cert_der[..])
                .map_err(|_| AuthenticationError::InvalidCredentials)?;
            let (_, x509) = X509Certificate::from_der(&cert_der).map_err(|_| {
                AuthenticationError::Internal(
                    "Failed to parse certificate with x509-parser".to_string(),
                )
            })?;
            let node_id = x509
                .subject()
                .iter_common_name()
                .next()
                .map(|cn| cn.as_str().unwrap_or("Unknown").to_string())
                .unwrap_or("Unknown".to_string());
            let node_type = x509
                .subject()
                .iter_organizational_unit()
                .next()
                .map(|ou| ou.as_str().unwrap_or("Unknown").into())
                .unwrap_or(NodeType::Unknown);

            let trust_anchors =
                TlsClientTrustAnchors(&[
                    TrustAnchor::try_from_cert_der(self.root_cert.contents()).unwrap()
                ]);

            let now = Time::from_seconds_since_unix_epoch(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            cert.verify_is_valid_tls_client_cert(SUPPORTED_SIG_ALGS, &trust_anchors, &[], now)
                .map_err(|e| {
                    AuthenticationError::Internal(format!("Cert verification failed: {e}"))
                })?;

            Ok(NodeMetadata {
                node_type,
                authenticated_using: AuthType::CertAuth,
                id: node_id,
            })
        } else {
            Err(AuthenticationError::InvalidCredentials)
        }
    }
}

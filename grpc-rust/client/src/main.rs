use hello_world::api_client::ApiClient;
use hello_world::{Message, ScalarValueTypes};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use std::error::Error;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tonic::Request;
use tonic::transport::{Channel, ClientTlsConfig};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Debug)]
struct NoCertificateVerification {}

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //let cert_file = fs::read_to_string("cert.pem")?;
    //let cert = Certificate::from_pem(cert_file);

    let tls_config = ClientTlsConfig::new();
    //.ca_certificate(cert);
    //.domain_name("localhost");

    let channel = Channel::from_static("https://127.0.0.1:5001")
        .tls_config_with_verifier(tls_config, Arc::new(NoCertificateVerification {}))?
        .connect()
        .await?;

    let mut client = ApiClient::new(channel);

    // Call
    let req = Request::new(ScalarValueTypes::default());
    let res = client.call(req).await?;
    println!("{:?}", res);

    // Download
    let req = Request::new(());
    let mut res = client.download(req).await?;
    let mut count = 0;
    while let Some(msg) = res.get_mut().message().await? {
        println!("Message: {:?}", msg);
        count += 1;

        if count > 3 {
            break;
        }
    }

    // Upload
    let req = tokio_stream::iter(0..3).map(|i| Message {
        message: format!("count {i}"),
    });
    let res = client.upload(req).await?;
    println!("{:?}", res);

    // Async
    let req = tokio_stream::iter(0..3).map(|i| Message {
        message: format!("count {i}"),
    });
    let mut res = client.r#async(req).await?;
    while let Some(msg) = res.get_mut().message().await? {
        println!("Message: {:?}", msg);
    }

    Ok(())
}

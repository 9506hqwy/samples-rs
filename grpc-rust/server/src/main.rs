use hello_world::api_server::{Api, ApiServer};
use hello_world::{Message, ScalarValueTypes};
use std::error::Error;
use std::fs;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status, Streaming};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Default)]
pub struct ApiImpl {}

#[tonic::async_trait]
impl Api for ApiImpl {
    type DownloadStream = Pin<Box<dyn Stream<Item = Result<Message, Status>> + Send>>;
    type AsyncStream = Pin<Box<dyn Stream<Item = Result<Message, Status>> + Send>>;

    async fn call(
        &self,
        request: Request<ScalarValueTypes>,
    ) -> std::result::Result<Response<ScalarValueTypes>, Status> {
        let req = request.get_ref();
        let res = req.clone();
        Ok(tonic::Response::new(res))
    }

    async fn download(
        &self,
        _: Request<()>,
    ) -> std::result::Result<Response<Self::DownloadStream>, Status> {
        let (tx, rx) = mpsc::channel(1);

        tokio::spawn(async move {
            let mut counter = 0;
            loop {
                time::sleep(Duration::from_secs(1)).await;

                let res = Ok(Message {
                    message: format!("count {counter}"),
                });

                match tx.send(res).await {
                    Ok(_) => {}
                    _ => {
                        break;
                    }
                }

                counter += 1;
            }
        });

        let stream = ReceiverStream::new(rx);
        let res = Response::new(Box::pin(stream) as Self::DownloadStream);
        Ok(res)
    }

    async fn upload(
        &self,
        request: Request<Streaming<Message>>,
    ) -> std::result::Result<Response<()>, Status> {
        let mut stream = request.into_inner();

        while let Some(msg) = stream.message().await? {
            println!("Upload: {}", msg.message);
        }

        Ok(Response::new(()))
    }

    async fn r#async(
        &self,
        request: Request<Streaming<Message>>,
    ) -> std::result::Result<Response<Self::AsyncStream>, Status> {
        let mut stream = request.into_inner();

        let (tx, rx) = mpsc::channel(1);

        tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                match tx.send(Ok(msg)).await {
                    Ok(_) => {}
                    _ => {
                        break;
                    }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        let res = Response::new(Box::pin(stream) as Self::AsyncStream);
        Ok(res)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cert = fs::read_to_string("cert.pem")?;
    let key = fs::read_to_string("key.pem")?;

    let http_addr = "0.0.0.0:5000".parse()?;
    let https_addr = "0.0.0.0:5001".parse()?;

    let service1 = ApiImpl::default();
    let service2 = ApiImpl::default();

    let http_server = Server::builder()
        .add_service(ApiServer::new(service1))
        .serve(http_addr);

    let https_server = Server::builder()
        .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(&cert, &key)))?
        .add_service(ApiServer::new(service2))
        .serve(https_addr);

    tokio::try_join!(http_server, https_server)?;
    Ok(())
}

mod ipfs_api_ext;
mod ipfs_fs;
mod metadata;
mod mfs;
mod read;
mod request_log;
mod rootfs;
mod write;

use crate::ipfs_fs::IpfsFs;
use crate::metadata::StaticMetadata;
use crate::mfs::MfsFs;
use crate::request_log::RequestLog;
use crate::rootfs::RootFs;
use futures::TryStreamExt;
use hyper::Request;
use ipfs_api_backend_hyper::{Error, IpfsApi, IpfsClient, TryFromUri};
use std::convert::Infallible;
use std::error::Error as _;
use std::net::SocketAddr;
use std::str::FromStr;
use unixfs_v1::dagpb::node_data;
use unixfs_v1::UnixFs;
use webdav_handler::davpath::DavPath;
use webdav_handler::fs::FsError;
use webdav_handler::memls::MemLs;
use webdav_handler::DavHandler;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .filter_or("IPFS_WEBDAV_LOG", "info")
            .write_style("IPFS_WEBDAV_LOG_STYLE"),
    )
    .init();

    let ipfs = std::env::var("IPFS_WEBDAV_API_ENDPOINT_URL")
        .ok()
        .map(|addr| {
            IpfsClient::from_str(&addr).unwrap_or_else(|e| {
                log::error!(
                    "Failed to parse IPFS API endpoint URL `{}` specified by \
                `IPFS_WEBDAV_API_ENDPOINT_URL` environment variable. It must be \
                in URL form. For example, `http://localhost:5001`. Error details: {}",
                    addr,
                    e
                );
                std::process::exit(101)
            })
        })
        .unwrap_or_else(|| IpfsClient::default());

    let addr: SocketAddr = std::env::var("IPFS_WEBDAV_LISTEN")
        .ok()
        .map(|addr| {
            SocketAddr::from_str(&addr).unwrap_or_else(|_| {
                log::error!(
                    "Failed to parse listen address `{}` specified by \
                `IPFS_WEBDAV_LISTEN` environment variable. It must be in `host:port` \
                form. For example, `localhost:4918` or `0.0.0.0:4918`",
                    addr
                );
                std::process::exit(101)
            })
        })
        .unwrap_or_else(|| ([127, 0, 0, 1], 4918).into());

    let dav_server = DavHandler::builder()
        .autoindex(true)
        .filesystem(Box::new(RootFs {
            mfs: MfsFs { ipfs: ipfs.clone() },
            ipfs: IpfsFs {
                ipfs: ipfs.clone(),
                ty: IpfsOrIpns::Ipfs,
            },
            ipns: IpfsFs {
                ipfs,
                ty: IpfsOrIpns::Ipns,
            },
        }))
        .locksystem(MemLs::new())
        .build_handler();

    let make_service = hyper::service::make_service_fn(move |_| {
        let dav_server = dav_server.clone();
        async move {
            let func = move |req: Request<_>| {
                let log = RequestLog::on_request(&req);
                let dav_server = dav_server.clone();
                async move {
                    let resp = dav_server.handle(req).await;
                    log.on_response(&resp);
                    Ok::<_, Infallible>(resp)
                }
            };
            Ok::<_, Infallible>(hyper::service::service_fn(func))
        }
    });

    let server = hyper::Server::try_bind(&addr).unwrap_or_else(|e| {
        match e.source() {
            None => log::error!("Error binding to {}: {}", addr, e),
            Some(s) => log::error!("Error binding to {}: {}", addr, s),
        }
        std::process::exit(101)
    });

    log::info!("Starting WebDAV server at http://{}", addr);

    let _ = server
        .serve(make_service)
        .await
        .map_err(|e| log::error!("server error: {}", e));
}

#[derive(Clone)]
pub enum IpfsOrIpns {
    Ipns,
    Ipfs,
}

async fn stat_metadata(ipfs: &IpfsClient, ipfs_path: &str) -> Result<StaticMetadata, FsError> {
    let response = ipfs
        .block_get(ipfs_path)
        .map_ok(|chunk| chunk.to_vec())
        .try_concat()
        .await;
    let block_bytes = response.map_err(handle_error)?;
    let unixfs_data_result =
        node_data(&block_bytes).and_then(|block_data| UnixFs::try_from(block_data));
    let data = match unixfs_data_result {
        Ok(data) => data,
        Err(_) => {
            return Ok(StaticMetadata {
                len: block_bytes.len() as u64,
                is_dir: false,
                is_unixfs: false,
            })
        }
    };
    Ok(StaticMetadata::from_unixfs_data(&data))
}

fn map_path(path: &DavPath) -> Result<&str, FsError> {
    std::str::from_utf8(path.as_bytes()).map_err(|_| FsError::GeneralFailure)
}

fn handle_error(e: Error) -> FsError {
    match &e {
        Error::Api(e) => {
            if e.code == 0
                && (e.message == "file does not exist" || e.message.starts_with("no link named "))
            {
                return FsError::NotFound;
            }
        }
        _ => {}
    };

    log::error!("Got an error from IPFS API: {}", e);

    return FsError::GeneralFailure;
}

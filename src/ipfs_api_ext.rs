use bytes::Bytes;
use futures::{future, FutureExt, Stream};
use ipfs_api_backend_hyper::request::ApiRequest;
use ipfs_api_backend_hyper::{Error, IpfsClient};
use ipfs_api_prelude::Backend;
use serde::Serialize;

pub trait IpfsClientExt {
    fn cat_with_options(
        &self,
        path: &str,
        offset: Option<i64>,
        length: Option<i64>,
    ) -> Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Unpin>;

    fn block_get_with_options(
        &self,
        path: &str,
        offset: Option<i64>,
        length: Option<i64>,
    ) -> Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Unpin>;
}

impl IpfsClientExt for IpfsClient {
    fn cat_with_options(
        &self,
        path: &str,
        offset: Option<i64>,
        length: Option<i64>,
    ) -> Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Unpin> {
        let req = Cat {
            path,
            offset,
            length,
        };
        match self.build_base_request(req, None) {
            Ok(req) => self.request_stream_bytes(req),
            Err(e) => Box::new(future::err(e).into_stream()),
        }
    }

    fn block_get_with_options(
        &self,
        hash: &str,
        offset: Option<i64>,
        length: Option<i64>,
    ) -> Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Unpin> {
        let req = BlockGet {
            hash,
            offset,
            length,
        };
        match self.build_base_request(req, None) {
            Ok(req) => self.request_stream_bytes(req),
            Err(e) => Box::new(future::err(e).into_stream()),
        }
    }
}

#[derive(Serialize)]
pub struct Cat<'a> {
    #[serde(rename = "arg")]
    pub path: &'a str,
    pub offset: Option<i64>,
    pub length: Option<i64>,
}

impl<'a> ApiRequest for Cat<'a> {
    const PATH: &'static str = "/cat";
}

#[derive(Serialize)]
pub struct BlockGet<'a> {
    #[serde(rename = "arg")]
    pub hash: &'a str,
    pub offset: Option<i64>,
    pub length: Option<i64>,
}

impl<'a> ApiRequest for BlockGet<'a> {
    const PATH: &'static str = "/block/get";
}

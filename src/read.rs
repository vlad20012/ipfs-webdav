use crate::ipfs_api_ext::IpfsClientExt;
use crate::{handle_error, StaticMetadata};
use futures::{future, FutureExt, Stream, StreamExt};
use hyper::body::{Buf, Bytes};
use ipfs_api_backend_hyper::{request, Error, IpfsApi, IpfsClient};
use std::fmt::{Debug, Formatter};
use std::io::SeekFrom;
use tokio::sync::Mutex;
use webdav_handler::fs::{DavFile, DavMetaData, FsError, FsFuture};

pub struct ReadOnlyDavFile {
    ipfs: IpfsClient,
    path: String,
    metadata: StaticMetadata,
    stream_supplier: fn(
        ipfs: &IpfsClient,
        metadata: &StaticMetadata,
        path: &str,
        seek: Option<i64>,
    ) -> Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Unpin>,
    seek: Option<i64>,
    stream: Mutex<Option<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Unpin>>>,
    rest: Option<Bytes>,
}

impl ReadOnlyDavFile {
    fn new(
        ipfs: IpfsClient,
        path: String,
        metadata: StaticMetadata,
        stream_supplier: fn(
            ipfs: &IpfsClient,
            metadata: &StaticMetadata,
            path: &str,
            seek: Option<i64>,
        ) -> Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Unpin>,
    ) -> Self {
        ReadOnlyDavFile {
            ipfs,
            path,
            metadata,
            stream_supplier,
            seek: None,
            stream: Mutex::new(None),
            rest: None,
        }
    }

    pub fn new_mfs(ipfs: IpfsClient, path: String, metadata: StaticMetadata) -> Self {
        Self::new(ipfs, path, metadata, |ipfs, _, path, seek| {
            ipfs.files_read_with_options(request::FilesRead {
                path: &path,
                offset: seek,
                ..request::FilesRead::default()
            })
        })
    }

    pub fn new_ipfs(ipfs: IpfsClient, path: String, metadata: StaticMetadata) -> Self {
        Self::new(ipfs, path, metadata, |ipfs, metadata, path, seek| {
            if metadata.is_unixfs {
                ipfs.cat_with_options(path, seek, None)
            } else {
                ipfs.block_get_with_options(path, seek, None)
            }
        })
    }
}

impl Debug for ReadOnlyDavFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadOnlyMfsFile")
    }
}

impl DavFile for ReadOnlyDavFile {
    fn metadata<'a>(&'a mut self) -> FsFuture<Box<dyn DavMetaData>> {
        Box::pin(future::ready(Ok(
            Box::new(self.metadata) as Box<dyn DavMetaData>
        )))
    }

    fn write_buf<'a>(&'a mut self, _: Box<dyn Buf + Send>) -> FsFuture<()> {
        Box::pin(future::ready(Err(FsError::NotImplemented)))
    }

    fn write_bytes(&mut self, _: Bytes) -> FsFuture<()> {
        Box::pin(future::ready(Err(FsError::NotImplemented)))
    }

    fn read_bytes(&mut self, count: usize) -> FsFuture<Bytes> {
        async move {
            if let Some(mut b) = self.rest.take() {
                if count < b.len() {
                    self.rest = Some(b.split_off(count));
                }
                return Ok(b);
            }
            let stream = self.stream.get_mut().get_or_insert_with(|| {
                (self.stream_supplier)(&self.ipfs, &self.metadata, &self.path, self.seek)
            });
            let next = match stream.next().await {
                Some(next) => next.map_err(handle_error),
                None => Err(FsError::GeneralFailure),
            };
            match next {
                Ok(mut b) => {
                    if count < b.len() {
                        self.rest = Some(b.split_off(count));
                    }
                    Ok(b)
                }
                Err(e) => Err(e),
            }
        }
        .boxed()
    }

    fn seek(&mut self, pos: SeekFrom) -> FsFuture<u64> {
        let start = match pos {
            SeekFrom::Start(start) => start,
            _ => panic!("seek must be SeekFrom::Start"),
        };
        self.seek = Some(i64::try_from(start).unwrap());
        *self.stream.get_mut() = None;
        self.rest = None;
        Box::pin(future::ready(Ok(start)))
    }

    fn flush(&mut self) -> FsFuture<()> {
        Box::pin(future::ready(Ok(())))
    }
}

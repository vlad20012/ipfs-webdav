use crate::{handle_error, StaticMetadata};
use common_multipart_rfc7578::client::multipart;
use futures::channel::mpsc::Sender;
use futures::{future, FutureExt, SinkExt, TryStreamExt};
use hyper::body::{Buf, Bytes};
use ipfs_api_backend_hyper::{Error, IpfsApi, IpfsClient};
use ipfs_api_prelude::request::FilesWrite;
use ipfs_api_prelude::Backend;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::SeekFrom;
use tokio::task::{JoinError, JoinHandle};
use webdav_handler::fs::{DavFile, DavMetaData, FsError, FsFuture};

pub struct WriteOnlyDavFile {
    ipfs: IpfsClient,
    path: String,
    create: bool,
    truncate: bool,
    stream: Option<Sender<io::Result<Bytes>>>,
    task: Option<JoinHandle<Result<(), Error>>>,
    len: u64,
    seek: Option<i64>,
}

impl WriteOnlyDavFile {
    pub fn new(ipfs: IpfsClient, path: String, create: bool, truncate: bool) -> Self {
        WriteOnlyDavFile {
            ipfs,
            path,
            create,
            truncate,
            stream: None,
            task: None,
            len: 0,
            seek: None,
        }
    }
}

impl Debug for WriteOnlyDavFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("WriteOnlyDavFile")
    }
}

impl DavFile for WriteOnlyDavFile {
    fn metadata<'a>(&'a mut self) -> FsFuture<Box<dyn DavMetaData>> {
        Box::pin(future::ready(Ok(
            Box::new(StaticMetadata::new_file(self.len)) as Box<dyn DavMetaData>,
        )))
    }

    fn write_buf<'a>(&'a mut self, _: Box<dyn Buf + Send>) -> FsFuture<()> {
        // The library actually does not call `write_buf`
        Box::pin(future::ready(Err(FsError::NotImplemented)))
    }

    fn write_bytes(&mut self, buf: Bytes) -> FsFuture<()> {
        async move {
            self.len += buf.len() as u64;
            let stream = self.stream.get_or_insert_with(|| {
                let (tx, rx) = futures::channel::mpsc::channel::<io::Result<Bytes>>(1);
                let mut form = multipart::Form::default();
                form.add_async_reader("data", rx.into_async_read());
                let ipfs = self.ipfs.clone();
                let path = self.path.clone();
                let create = self.create;
                let truncate = self.truncate;
                let seek = self.seek.clone();
                self.task = Some(tokio::spawn(async move {
                    let req = FilesWrite {
                        path: &path,
                        create: Some(create),
                        truncate: Some(truncate),
                        offset: seek,
                        ..Default::default()
                    };
                    ipfs.request_empty(req, Some(form)).await
                }));
                tx
            });
            match stream.send(Ok(buf)).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("A `write` aborted with error: {}", e);
                    return Err(FsError::GeneralFailure);
                }
            }
            Ok(())
        }
        .boxed()
    }

    fn read_bytes(&mut self, _: usize) -> FsFuture<Bytes> {
        Box::pin(future::ready(Err(FsError::NotImplemented)))
    }

    fn seek(&mut self, pos: SeekFrom) -> FsFuture<u64> {
        // The library actually calls `seek` only once during PUT
        assert!(self.seek.is_none());
        let start = match pos {
            SeekFrom::Start(start) => start,
            _ => panic!("seek must be SeekFrom::Start"),
        };
        self.seek = Some(i64::try_from(start).unwrap());
        Box::pin(future::ready(Ok(start)))
    }

    fn flush(&mut self) -> FsFuture<()> {
        async move {
            if self.stream.is_none() && self.len == 0 {
                self.ipfs
                    .files_write(&self.path, self.create, self.truncate, &[] as &[u8])
                    .await
                    .map_err(handle_error)?;
            }
            match self.stream.take() {
                Some(mut s) => s.close().await.expect(
                    "This error must not happen because \
                    `Sender::close` seems infallible",
                ),
                None => {}
            }
            match self.task.take() {
                Some(t) => t
                    .await
                    .map_err(handle_join_error)
                    .and_then(|ok| ok.map_err(handle_error))?,
                None => {}
            }
            Ok(())
        }
        .boxed()
    }
}

fn handle_join_error(e: JoinError) -> FsError {
    if e.is_cancelled() {
        log::error!("A `write` task was canceled unexpectedly")
    } else if e.is_panic() {
        // Do nothing because the panic is already logged
    } else {
        // Actually unreachable (but it can be changed in future tokio versions)
        log::error!(
            "An unexpected error occurred during `write` request: {:?}",
            e
        )
    }
    return FsError::GeneralFailure;
}

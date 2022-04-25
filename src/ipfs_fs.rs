use crate::metadata::{DynamicLsEntry, StaticLsEntry, StaticMetadata};
use crate::read::ReadOnlyDavFile;
use crate::{handle_error, map_path, stat_metadata, IpfsOrIpns};
use futures::{future, stream, FutureExt};
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient};
use webdav_handler::davpath::DavPath;
use webdav_handler::fs::{
    DavDirEntry, DavFile, DavFileSystem, DavMetaData, FsError, FsFuture, FsStream, OpenOptions,
    ReadDirMeta,
};

#[derive(Clone)]
pub struct IpfsFs {
    pub ipfs: IpfsClient,
    pub ty: IpfsOrIpns,
}

impl DavFileSystem for IpfsFs {
    fn open<'a>(&'a self, path: &'a DavPath, options: OpenOptions) -> FsFuture<Box<dyn DavFile>> {
        return if path.as_bytes() == &[b'/'] {
            Box::pin(future::ready(Err(FsError::Forbidden)))
        } else {
            async move {
                if options.write {
                    return Err(FsError::Forbidden);
                }
                let ipfs_path = self.to_ipfs_path(path)?;
                let stat = self.stat_metadata(&ipfs_path).await?;
                let file =
                    ReadOnlyDavFile::new_ipfs(self.ipfs.clone(), ipfs_path.to_string(), stat);
                Ok(Box::new(file) as Box<dyn DavFile>)
            }
            .boxed()
        };
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a DavPath,
        _: ReadDirMeta,
    ) -> FsFuture<FsStream<Box<dyn DavDirEntry>>> {
        async move {
            if path.as_bytes() == &[b'/'] {
                let dirs: Vec<_> = match self.ty {
                    IpfsOrIpns::Ipns => {
                        let keys = self.ipfs.key_list().await.map_err(handle_error)?;
                        keys.keys
                            .into_iter()
                            .map(move |k| {
                                Box::new(DynamicLsEntry {
                                    ipfs: self.ipfs.clone(),
                                    ty: IpfsOrIpns::Ipns,
                                    cid: k.id.to_string(),
                                }) as Box<dyn DavDirEntry>
                            })
                            .collect()
                    }
                    IpfsOrIpns::Ipfs => {
                        let pins1 = self
                            .ipfs
                            .pin_ls(None, Some("recursive"))
                            .await
                            .map_err(handle_error)?;
                        let pins2 = self
                            .ipfs
                            .pin_ls(None, Some("direct"))
                            .await
                            .map_err(handle_error)?;
                        pins1
                            .keys
                            .into_iter()
                            .chain(pins2.keys.into_iter())
                            .map(move |(cid, _)| {
                                Box::new(DynamicLsEntry {
                                    ipfs: self.ipfs.clone(),
                                    ty: IpfsOrIpns::Ipfs,
                                    cid,
                                }) as Box<dyn DavDirEntry>
                            })
                            .collect()
                    }
                };
                let stream = Box::pin(stream::iter(dirs)) as FsStream<Box<dyn DavDirEntry>>;
                Ok(stream)
            } else {
                let ipfs_path = self.to_ipfs_path(path)?;
                let ls = self.ipfs.ls(&ipfs_path).await.map_err(handle_error)?;
                let f = match ls.objects.into_iter().next() {
                    None => return Err(FsError::NotFound),
                    Some(f) => f,
                };
                let entries = f.links.into_iter().map(move |e| {
                    Box::new(StaticLsEntry::from_ipfs_file_header(e)) as Box<dyn DavDirEntry>
                });

                Ok(Box::pin(stream::iter(entries)) as FsStream<Box<dyn DavDirEntry>>)
            }
        }
        .boxed()
    }

    fn metadata<'a>(&'a self, path: &'a DavPath) -> FsFuture<Box<dyn DavMetaData>> {
        return if path.as_bytes() == &[b'/'] {
            Box::pin(future::ready(Ok(
                Box::new(StaticMetadata::new_dir()) as Box<dyn DavMetaData>
            )))
        } else {
            async move {
                let ipfs_path = self.to_ipfs_path(path)?;
                Ok(Box::new(self.stat_metadata(&ipfs_path).await?) as Box<dyn DavMetaData>)
            }
            .boxed()
        };
    }
}

impl IpfsFs {
    async fn stat_metadata(&self, ipfs_path: &str) -> Result<StaticMetadata, FsError> {
        stat_metadata(&self.ipfs, ipfs_path).await
    }

    fn to_ipfs_path(&self, path: &DavPath) -> Result<String, FsError> {
        let prefix = match self.ty {
            IpfsOrIpns::Ipns => "/ipns",
            IpfsOrIpns::Ipfs => "/ipfs",
        };
        let mut ipfs_path = prefix.to_string();
        ipfs_path += map_path(path)?;
        Ok(ipfs_path)
    }
}

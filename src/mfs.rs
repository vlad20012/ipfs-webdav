use crate::metadata::{StaticLsEntry, StaticMetadata};
use crate::read::ReadOnlyDavFile;
use crate::write::WriteOnlyDavFile;
use crate::{handle_error, map_path};
use futures::{stream, FutureExt};
use ipfs_api_backend_hyper::{request, IpfsApi, IpfsClient};
use webdav_handler::davpath::DavPath;
use webdav_handler::fs::{
    DavDirEntry, DavFile, DavFileSystem, DavMetaData, FsError, FsFuture, FsStream, OpenOptions,
    ReadDirMeta,
};

#[derive(Clone)]
pub struct MfsFs {
    pub ipfs: IpfsClient,
}

impl DavFileSystem for MfsFs {
    fn open<'a>(&'a self, path: &'a DavPath, options: OpenOptions) -> FsFuture<Box<dyn DavFile>> {
        async move {
            let path = map_path(path)?;
            if options.write {
                let file = WriteOnlyDavFile::new(
                    self.ipfs.clone(),
                    path.to_string(),
                    options.create || options.create_new,
                    options.truncate,
                );
                Ok(Box::new(file) as Box<dyn DavFile>)
            } else {
                let stat = self.stat_metadata(path).await?;
                let file = ReadOnlyDavFile::new_mfs(self.ipfs.clone(), path.to_string(), stat);
                Ok(Box::new(file) as Box<dyn DavFile>)
            }
        }
        .boxed()
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a DavPath,
        _: ReadDirMeta,
    ) -> FsFuture<FsStream<Box<dyn DavDirEntry>>> {
        async move {
            let path = map_path(path)?;
            let ls = self
                .ipfs
                .files_ls_with_options(request::FilesLs {
                    path: Some(path),
                    long: Some(true),
                    unsorted: None,
                })
                .await
                .map_err(handle_error)?;

            let dirs = ls
                .entries
                .into_iter()
                .map(move |e| Box::new(StaticLsEntry::from_files_entry(e)) as Box<dyn DavDirEntry>);

            Ok(Box::pin(stream::iter(dirs)) as FsStream<Box<dyn DavDirEntry>>)
        }
        .boxed()
    }

    fn metadata<'a>(&'a self, path: &'a DavPath) -> FsFuture<Box<dyn DavMetaData>> {
        async {
            let path = map_path(path)?;
            Ok(Box::new(self.stat_metadata(path).await?) as Box<dyn DavMetaData>)
        }
        .boxed()
    }

    fn create_dir<'a>(&'a self, path: &'a DavPath) -> FsFuture<()> {
        async {
            let path = map_path(path)?;
            // TODO Exists, NotFound
            self.ipfs
                .files_mkdir(path, false)
                .await
                .map_err(handle_error)?;
            Ok(())
        }
        .boxed()
    }

    fn remove_dir<'a>(&'a self, path: &'a DavPath) -> FsFuture<()> {
        async {
            let path = map_path(path)?;
            self.ipfs.files_rm(path, true).await.map_err(handle_error)?;
            Ok(())
        }
        .boxed()
    }

    fn remove_file<'a>(&'a self, path: &'a DavPath) -> FsFuture<()> {
        async {
            let path = map_path(path)?;
            self.ipfs
                .files_rm(path, false)
                .await
                .map_err(handle_error)?;
            Ok(())
        }
        .boxed()
    }

    fn rename<'a>(&'a self, from: &'a DavPath, to: &'a DavPath) -> FsFuture<()> {
        async {
            if from.as_bytes() == &[b'/'] {
                Err(FsError::Forbidden)
            } else if from.as_bytes().starts_with(b"/mfs") {
                let mut from = from.clone();
                from.set_prefix("/mfs").unwrap();
                let mut from = map_path(&from)?;
                if from.ends_with("/") {
                    from = &from[0..from.len() - 1];
                }
                self.ipfs
                    .files_mv(from, map_path(to)?)
                    .await
                    .map_err(handle_error)
            } else if from.as_bytes().starts_with(b"/ipfs") || from.as_bytes().starts_with(b"/ipns")
            {
                let mut from = map_path(from)?;
                if from.ends_with("/") {
                    from = &from[0..from.len() - 1];
                }
                self.ipfs
                    .files_cp(from, map_path(to)?)
                    .await
                    .map_err(handle_error)
            } else {
                Err(FsError::Forbidden)
            }
        }
        .boxed()
    }

    fn copy<'a>(&'a self, from: &'a DavPath, to: &'a DavPath) -> FsFuture<()> {
        async {
            if from.as_bytes() == &[b'/'] {
                Err(FsError::Forbidden)
            } else if from.as_bytes().starts_with(b"/mfs") {
                let mut from = from.clone();
                from.set_prefix("/mfs").unwrap();
                let mut from = map_path(&from)?;
                if from.ends_with("/") {
                    from = &from[0..from.len() - 1];
                }
                self.ipfs
                    .files_cp(from, map_path(to)?)
                    .await
                    .map_err(handle_error)
            } else if from.as_bytes().starts_with(b"/ipfs") || from.as_bytes().starts_with(b"/ipns")
            {
                let mut from = map_path(from)?;
                if from.ends_with("/") {
                    from = &from[0..from.len() - 1];
                }
                self.ipfs
                    .files_cp(from, map_path(to)?)
                    .await
                    .map_err(handle_error)
            } else {
                Err(FsError::Forbidden)
            }
        }
        .boxed()
    }
}

impl MfsFs {
    async fn stat_metadata(&self, path: &str) -> Result<StaticMetadata, FsError> {
        let stat = self.ipfs.files_stat(path).await.map_err(handle_error)?;
        Ok(StaticMetadata::from_files_stat_response(stat))
    }
}

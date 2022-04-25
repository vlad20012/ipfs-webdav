use crate::ipfs_fs::IpfsFs;
use crate::metadata::{StaticLsEntry, StaticMetadata};
use crate::mfs::MfsFs;
use futures::{stream, FutureExt};
use hyper::StatusCode;
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;
use webdav_handler::davpath::DavPath;
use webdav_handler::fs::{
    DavDirEntry, DavFile, DavFileSystem, DavMetaData, DavProp, FsError, FsFuture, FsStream,
    OpenOptions, ReadDirMeta,
};

#[derive(Clone)]
pub struct RootFs {
    pub mfs: MfsFs,
    pub ipfs: IpfsFs,
    pub ipns: IpfsFs,
}

enum FsKind<'a> {
    Root,
    Provided(&'a dyn DavFileSystem, DavPath),
    Unknown,
}

impl RootFs {
    fn lookup_fs(&self, path: &DavPath) -> FsKind {
        if path.as_bytes() == &[b'/'] {
            FsKind::Root
        } else if path.as_bytes().starts_with(b"/mfs") {
            let mut next_path = path.clone();
            next_path.set_prefix("/mfs").unwrap();
            FsKind::Provided(&self.mfs, next_path)
        } else if path.as_bytes().starts_with(b"/ipfs") {
            let mut next_path = path.clone();
            next_path.set_prefix("/ipfs").unwrap();
            FsKind::Provided(&self.ipfs, next_path)
        } else if path.as_bytes().starts_with(b"/ipns") {
            let mut next_path = path.clone();
            next_path.set_prefix("/ipns").unwrap();
            FsKind::Provided(&self.ipns, next_path)
        } else {
            FsKind::Unknown
        }
    }
}

impl DavFileSystem for RootFs {
    fn open<'a>(&'a self, path: &'a DavPath, options: OpenOptions) -> FsFuture<Box<dyn DavFile>> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Root => Err(FsError::Forbidden),
                FsKind::Provided(fs, next_path) => fs.open(&next_path, options).await,
                FsKind::Unknown => Err(FsError::NotFound),
            }
        }
        .boxed()
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a DavPath,
        meta: ReadDirMeta,
    ) -> FsFuture<FsStream<Box<dyn DavDirEntry>>> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Root => {
                    let dirs = ["mfs", "ipfs", "ipns"].into_iter().map(|name| {
                        Box::new(StaticLsEntry::new_dir(name.to_string())) as Box<dyn DavDirEntry>
                    });
                    Ok(Box::pin(stream::iter(dirs)) as FsStream<Box<dyn DavDirEntry>>)
                }
                FsKind::Provided(fs, next_path) => fs.read_dir(&next_path, meta).await,
                FsKind::Unknown => Err(FsError::NotFound),
            }
        }
        .boxed()
    }

    fn metadata<'a>(&'a self, path: &'a DavPath) -> FsFuture<Box<dyn DavMetaData>> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Root => Ok(Box::new(StaticMetadata::new_dir()) as Box<dyn DavMetaData>),
                FsKind::Provided(fs, next_path) => fs.metadata(&next_path).await,
                FsKind::Unknown => Err(FsError::NotFound),
            }
        }
        .boxed()
    }

    fn symlink_metadata<'a>(&'a self, path: &'a DavPath) -> FsFuture<Box<dyn DavMetaData>> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Root => self.metadata(path).await,
                FsKind::Provided(fs, next_path) => fs.symlink_metadata(&next_path).await,
                FsKind::Unknown => Err(FsError::NotFound),
            }
        }
        .boxed()
    }

    fn create_dir<'a>(&'a self, path: &'a DavPath) -> FsFuture<()> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.create_dir(&next_path).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn remove_dir<'a>(&'a self, path: &'a DavPath) -> FsFuture<()> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.remove_dir(&next_path).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn remove_file<'a>(&'a self, path: &'a DavPath) -> FsFuture<()> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.remove_file(&next_path).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn rename<'a>(&'a self, from: &'a DavPath, to: &'a DavPath) -> FsFuture<()> {
        async move {
            match self.lookup_fs(to) {
                FsKind::Provided(fs, next_path) => fs.rename(from, &next_path).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn copy<'a>(&'a self, from: &'a DavPath, to: &'a DavPath) -> FsFuture<()> {
        async move {
            match self.lookup_fs(to) {
                FsKind::Provided(fs, next_path) => fs.copy(from, &next_path).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn set_accessed<'a>(&'a self, path: &'a DavPath, tm: SystemTime) -> FsFuture<()> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.set_accessed(&next_path, tm).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn set_modified<'a>(&'a self, path: &'a DavPath, tm: SystemTime) -> FsFuture<()> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.set_modified(&next_path, tm).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn have_props<'a>(
        &'a self,
        path: &'a DavPath,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.have_props(&next_path).await,
                FsKind::Root | FsKind::Unknown => false,
            }
        }
        .boxed()
    }

    fn patch_props<'a>(
        &'a self,
        path: &'a DavPath,
        patch: Vec<(bool, DavProp)>,
    ) -> FsFuture<Vec<(StatusCode, DavProp)>> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.patch_props(&next_path, patch).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn get_props<'a>(&'a self, path: &'a DavPath, do_content: bool) -> FsFuture<Vec<DavProp>> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.get_props(&next_path, do_content).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }

    fn get_prop<'a>(&'a self, path: &'a DavPath, prop: DavProp) -> FsFuture<Vec<u8>> {
        async move {
            match self.lookup_fs(path) {
                FsKind::Provided(fs, next_path) => fs.get_prop(&next_path, prop).await,
                FsKind::Root | FsKind::Unknown => Err(FsError::Forbidden),
            }
        }
        .boxed()
    }
}

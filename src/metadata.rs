use crate::{stat_metadata, IpfsOrIpns};
use futures::FutureExt;
use ipfs_api_backend_hyper::IpfsClient;
use ipfs_api_prelude::response::{FilesEntry, FilesStatResponse, IpfsFileHeader};
use std::fmt::Debug;
use std::time::SystemTime;
use unixfs_v1::{UnixFs, UnixFsType};
use webdav_handler::fs::{DavDirEntry, DavMetaData, FsFuture, FsResult};

#[derive(Clone, Copy, Debug)]
pub struct StaticMetadata {
    pub len: u64,
    pub is_dir: bool,
    pub is_unixfs: bool,
}

impl StaticMetadata {
    pub fn new_dir() -> StaticMetadata {
        StaticMetadata {
            len: 0,
            is_dir: true,
            is_unixfs: true,
        }
    }

    pub fn new_file(len: u64) -> StaticMetadata {
        StaticMetadata {
            len,
            is_dir: false,
            is_unixfs: true,
        }
    }

    pub fn from_unixfs_data(data: &UnixFs) -> StaticMetadata {
        if data.Type == UnixFsType::Directory || data.Type == UnixFsType::HAMTShard {
            StaticMetadata::new_dir()
        } else {
            StaticMetadata::new_file(data.filesize.unwrap())
        }
    }

    pub fn from_files_stat_response(stat: FilesStatResponse) -> StaticMetadata {
        StaticMetadata {
            len: stat.size,
            is_dir: stat.typ == "directory",
            is_unixfs: true,
        }
    }
}

impl DavMetaData for StaticMetadata {
    fn len(&self) -> u64 {
        self.len
    }

    fn modified(&self) -> FsResult<SystemTime> {
        Ok(SystemTime::UNIX_EPOCH)
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }

    fn executable(&self) -> FsResult<bool> {
        Ok(true)
    }
}

pub struct StaticLsEntry {
    name: String,
    len: u64,
    is_dir: bool,
}

impl StaticLsEntry {
    pub fn new_dir(name: String) -> Self {
        StaticLsEntry {
            name,
            len: 0,
            is_dir: true,
        }
    }

    pub fn from_files_entry(entry: FilesEntry) -> Self {
        StaticLsEntry {
            name: entry.name,
            len: entry.size,
            is_dir: entry.typ == 1,
        }
    }

    pub fn from_ipfs_file_header(entry: IpfsFileHeader) -> Self {
        StaticLsEntry {
            name: entry.name,
            len: entry.size,
            is_dir: entry.typ == 1,
        }
    }
}

impl DavDirEntry for StaticLsEntry {
    fn name(&self) -> Vec<u8> {
        self.name.clone().into_bytes()
    }

    fn metadata<'a>(&'a self) -> FsFuture<Box<dyn DavMetaData>> {
        async {
            // let mut path = "/ipns/";
            // path += name;
            // self.ipfs.ls(&path).await;
            // let stat = ipfs.files_stat(std::str::from_utf8(path.as_bytes()).unwrap()).await.map_err(|e| FsError::GeneralFailure)?;
            Ok(Box::new(StaticMetadata {
                len: self.len,
                is_dir: self.is_dir,
                is_unixfs: true,
            }) as Box<dyn DavMetaData>)
        }
        .boxed()
    }
}

pub struct DynamicLsEntry {
    pub ipfs: IpfsClient,
    pub ty: IpfsOrIpns,
    pub cid: String,
}

impl DavDirEntry for DynamicLsEntry {
    fn name(&self) -> Vec<u8> {
        self.cid.clone().into_bytes()
    }

    fn metadata<'a>(&'a self) -> FsFuture<Box<dyn DavMetaData>> {
        async {
            let prefix = match self.ty {
                IpfsOrIpns::Ipns => "/ipns/",
                IpfsOrIpns::Ipfs => "/ipfs/",
            };
            let ipfs_path: String = [prefix, &self.cid].into_iter().collect();
            let metadata = stat_metadata(&self.ipfs, &ipfs_path).await?;
            Ok(Box::new(metadata) as Box<dyn DavMetaData>)
        }
        .boxed()
    }
}

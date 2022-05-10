# IPFS-WebDAV

IPFS-WebDAV is a daemon that uses [IPFS] HTTP RPC and exposes [WebDAV] API. WebDAV, in turn, can be used
to mount the IPFS file system in most operating systems.

```
Linux   -------------->   +-------------+           IPFS HTTP RPC           +------+
Mac OS  -----WebDAV-----> | IPFS-WebDAV | --------------------------------> | IPFS |
Windows -------------->   +-------------+   (e.g. http://localhost:5001)    +------+
```

Currently, IPFS-WebDAV exposes `/ipfs` and `/ipns` namespaces in read-only mode, and `/mfs` namespace
(referring to [MFS]) in read-write mode. The read-write access for `/ipns` is planned.

[IPFS]: https://ipfs.io
[WebDAV]: https://en.wikipedia.org/wiki/WebDAV
[MFS]: https://docs.ipfs.io/concepts/file-systems/

## Installation

### Docker

[![Docker Image Version (latest semver)](https://img.shields.io/docker/v/vlad20012/ipfs-webdav?arch=amd64&color=blue&label=ipfs-webdav%20docker%20image&sort=date)](https://hub.docker.com/r/vlad20012/ipfs-webdav)

Follow the guide on the [official docker image page](https://hub.docker.com/r/vlad20012/ipfs-webdav).

### Installation From Source

Run `cargo build --release`, then find the binary in `target/release/ipfs-webdav`

## License

The IPFS-WebDAV project is dual-licensed under Apache 2.0 and MIT terms:

- Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/vlad20012/ipfs-webdav/blob/master/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](https://github.com/vlad20012/ipfs-webdav/blob/master/LICENSE-MIT) or http://opensource.org/licenses/MIT)

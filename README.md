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

### Installation from binary distribution

You can find `ipfs-webdav` binaries on the [releases page](https://github.com/vlad20012/ipfs-webdav/releases).
Then just run it (`./ipfs-webdav_linux-x86-64`)

### Installation From Source

Run `cargo build --release`, then find the binary in `target/release/ipfs-webdav`

## Configuration

Currently, `ipfs-webdav` can be configured only using environment variables.
- `IPFS_WEBDAV_API_ENDPOINT_URL` - specifies URL of an IPFS RPC. For example, `http://localhost:5001`.
  If not specified, `ipfs-webdav` tries to read the URL from `~/.ipfs/api` file and falls back to
  `http://localhost:5001` if the file is not found.
- `IPFS_WEBDAV_LISTEN` - Specifies listen WebDAV address in `host:port` format. 
  For example, `localhost:4918` or `0.0.0.0:4918`. Default value is `127.0.0.1:4918`
- `IPFS_WEBDAV_LOG` - specifies `ipfs-webdav` log level. Possible values: `error`, `warn`, `info`, `debug`, `trace`.
  Default value is `info`.

Example: `IPFS_WEBDAV_API_ENDPOINT_URL="http://localhost:5001" IPFS_WEBDAV_LISTEN="0.0.0.0:4918" ./ipfs-webdav`

## Mounting WebDAV filesystem

### Linux

The simplest way to connect WebDAV on Linux with GNOME is connecting from `Nautilus` (aka `Files`).
Open Nautilus, select "+ Other Locations" on the left sidebar then type your server address in the
"Connect to Server" field. The address is usually `dav://localhost:4918/` or whatever you specified
using `IPFS_WEBDAV_LISTEN` environment variable. Then press `Connect` button.

For long-living mounts and for other Linux environments I suggest using `davfs2`:
```bash
mount -t davfs http://127.0.0.1:4918 /mnt
```
It asks to enter username and then password, just press `<enter>` instead.

### MacOS

On macOS, you can simply connect WebDAV drive from `Finder`. Follow the 
[official guide](https://support.apple.com/guide/mac-help/connect-disconnect-a-webdav-server-mac-mchlp1546/mac).
Use `http://localhost:4918/` (or whatever you specified using `IPFS_WEBDAV_LISTEN` environment variable) as
"Server Address".

### Windows

On Windows, you can simply connect WebDAV drive from `File Explorer` using `Map network drive...`
option. Use `http://localhost:4918/` (or whatever you specified using `IPFS_WEBDAV_LISTEN` environment variable) as 
"Internet or network address".

## License

The IPFS-WebDAV project is dual-licensed under Apache 2.0 and MIT terms:

- Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/vlad20012/ipfs-webdav/blob/master/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](https://github.com/vlad20012/ipfs-webdav/blob/master/LICENSE-MIT) or http://opensource.org/licenses/MIT)

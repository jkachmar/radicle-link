// This file is part of radicle-link
// <https://github.com/radicle-dev/radicle-link>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! A custom git transport
//!
//! The `register` function registers a transport which expects URLs of the
//! form:
//!
//! NOTE: Tells libgit2 to lookup the function pointer global var thing.
//! This will let libgit2 to call you back with this URL string.
//! You parse the URL and then you call open_stream(local_peer, remote_peer)
//! on Protocol.
//!
//! FIXME: Need to cram a bit of info, so that instead of looking for an
//! existing connection, it should establish a new one.
//! Otherwise, we change the impl of GitStreamFactory such that it always
//! makes a new connection.
//!
//! `rad-p2p://LOCAL_PEER_ID@REMOTE_PEER_ID/PROJECT_ID`
//!
//! NOTE: I want: rad-p2p://PROJECT_ID
//! 1. What I need for git-clone is to find a peer who has this.
//! 2. Only way is to do `query`.
//! 3. But if we already did `query`, we might already know who has this!
//! 4. We should avoid querying if we already know!
//! 5. Either need: (a) storage to keep track of this, or (b) let the user
//! choose whether to query or if they have a peer to try.
//! 6. For this to work, the url should be:
//!
//!     rad-p2p://REMOTE_PEER_ID[.SockerAddr]/Project_Id
//!
//!     NEED to know PeerId to establish connection.
//!
//! NOTE: `Fetcher` is what takes this URL as input
//!
//! The local peer id is needed to support testing with multiple peers:
//! `libgit2` stores custom transports in a `static` variable, so we can
//! register ours only once per program.
//!
//! # Note
//!
//! The wire protocol of the transport conforms to the one [`git-daemon`]
//! implements. However, there appears to be a bug in either `libgit2` or
//! `git2-rs` which prevents us from registering as a stateful transport:
//! apparently, the subtransport is instantiated twice, when it should only be
//! instantiated once, causing this assertion to fail:
//!
//! `libgit2/src/transports/smart.c:349: git_smart__negotiation_step: Assertion
//! `t->rpc || t->current_stream == stream' failed.`
//!
//! To work around this, we pretend to implement a stateless protocol by
//! indicating in the header line whether we want the remote side to only
//! advertise the refs, or wait for our haves. Of course, this makes this
//! transport incompatible with [`git-daemon`] for now, so the other side
//! needs to run our own [`GitServer`].
//!
//! [`git-daemon`]: https://git-scm.com/docs/git-daemon
//! [`GitServer`]: ../server/struct.GitServer.html

use std::{
    collections::HashMap,
    fmt::Display,
    io::{self, Read, Write},
    net::SocketAddr,
    sync::{Arc, Once, RwLock},
};

use futures::{
    executor::block_on,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
};
use git2::transport::{Service, SmartSubtransport, SmartSubtransportStream, Transport};

use crate::{
    git::{ext::into_git_err, header::Header, p2p::url::GitUrl},
    peer::PeerId,
    uri::{self, RadUrn},
};

type Factories = Arc<RwLock<HashMap<PeerId, Box<dyn GitStreamFactory>>>>;

// Global stream lookup. It's a hashmap, because we need to support multiple
// peers. One stream per peer. This is ONLY A HASHMAP FOR TESTS. OTHERWISE YOU
// NEED TO REGISTER ONLY ONCE - This happens under the hood.
//
// NOTE: Check the can_clone test.
lazy_static! {
    static ref FACTORIES: Factories = Arc::new(RwLock::new(HashMap::with_capacity(1)));
}

/// The underlying [`AsyncRead`] + [`AsyncWrite`] of a [`RadSubTransport`]
///
/// We need this as a trait because we can't write `Box<dyn AsyncRead +
/// AsyncWrite + Unpin + Send>` directly.
pub trait GitStream: AsyncRead + AsyncWrite + Unpin + Send {}

/// Trait for types which can provide a [`GitStream`] over which we can send
/// / receive bytes to / from the specified peer.
///
/// NOTE: Getting libgit2 to use our p2p stack as the transport.
///
/// If you register a transport with the same scheme twice, you get an error.
/// Therefore we must register a global thing that when called, it looks up
/// the stream from somewhere.
#[async_trait]
pub trait GitStreamFactory: Sync + Send {
    // This is how we open a stream that libgit2 is happy with.
    async fn open_stream(
        &self,
        to: &PeerId,
        addr: Option<SocketAddr>,
    ) -> Option<Box<dyn GitStream>>;
}

/// Register the `rad-p2p://` transport with `libgit`.
///
/// # Safety:
///
/// The actual register call to `libgit` is guarded by [`Once`], it is thus safe
/// to call this function multiple times -- subsequent calls will return a new
/// [`RadTransport`], which can be used to register additional stream factories.
///
/// The first call to this function MUST, however, be externally synchronised
/// with all other calls to `libgit`.
///
/// Call `register` anytime. Not only once. Makes sure it registers the
/// transport thing once, and returns the transport struct which contains the
/// global variable, ie. the Git stream. You can use this to register your
/// peer-id + the stream.
pub fn register() -> RadTransport {
    static INIT: Once = Once::new();

    unsafe {
        INIT.call_once(|| {
            git2::transport::register(super::URL_SCHEME, move |remote| {
                Transport::smart(&remote, true, RadTransport::new())
            })
            .unwrap();
        })
    }

    RadTransport::new()
}

#[derive(Clone)]
pub struct RadTransport {
    fac: Factories,
}

impl RadTransport {
    fn new() -> Self {
        Self {
            fac: FACTORIES.clone(),
        }
    }

    /// Register an additional [`GitStreamFactory`], which can open git streams
    /// on behalf of `peer_id`.
    ///
    /// See the module documentation for why we key stream factories by sender.
    pub fn register_stream_factory(&self, peer_id: &PeerId, fac: Box<dyn GitStreamFactory>) {
        self.fac.write().unwrap().insert(peer_id.clone(), fac);
    }

    fn open_stream<Addr>(
        &self,
        from: &PeerId,
        to: &PeerId,
        addr: Addr,
    ) -> Option<Box<dyn GitStream>>
    where
        Addr: Into<Option<SocketAddr>>,
    {
        self.fac
            .read()
            .unwrap()
            .get(from)
            .and_then(|fac| block_on(fac.open_stream(to, addr.into())))
    }
}

impl SmartSubtransport for RadTransport {
    // NOTE: Entry point for `git clone`.
    // We call open_stream with the info in the rad-p2p URL.
    // But we could also say that `GitStreamFactor::open_stream` also knows
    // the repo URL.
    // NOTE: This is the callback libgit2 calls and passes the special URL.
    //
    // Ideally when resolving happens, it persists the peer-id -> addr pair
    // somewhere, so it doesn't have to re-lookup every time.
    //
    // If we know the PeerId, we may _ALREADY_ know the Address, so sometimes
    // you do know the Address, even though it wouldn't be cached here!
    //
    // So for now, stick it at the highest level, eg. PeerApi.
    fn action(
        &self,
        url: &str,
        service: Service,
    ) -> Result<Box<dyn SmartSubtransportStream>, git2::Error> {
        let url: GitUrl = url.parse().map_err(into_git_err)?;
        let stream = self
            .open_stream(&url.local_peer, &url.remote_peer, url.remote_addr)
            .ok_or_else(|| into_git_err(format!("No connection to {}", url.remote_peer)))?;

        Ok(Box::new(RadSubTransport {
            header_sent: false,
            url,
            service,
            stream,
        }))
    }

    fn close(&self) -> Result<(), git2::Error> {
        Ok(())
    }
}

struct RadSubTransport {
    header_sent: bool,
    url: GitUrl,
    service: Service,
    stream: Box<dyn GitStream>,
}

impl RadSubTransport {
    async fn ensure_header_sent(&mut self) -> io::Result<()> {
        if !self.header_sent {
            self.header_sent = true;
            let header = Header::new(
                self.service,
                RadUrn::new(
                    self.url.repo.clone(),
                    uri::Protocol::Git,
                    uri::Path::empty(),
                ),
                self.url.remote_peer.clone(),
            );
            self.stream.write_all(header.to_string().as_bytes()).await
        } else {
            Ok(())
        }
    }
}

impl Read for RadSubTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        block_on(async {
            self.ensure_header_sent().await?;
            self.stream.read(buf).await.map_err(io_error)
        })
    }
}

impl Write for RadSubTransport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        block_on(async {
            self.ensure_header_sent().await?;
            self.stream.write(buf).await.map_err(io_error)
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        block_on(async {
            self.ensure_header_sent().await?;
            self.stream.flush().await.map_err(io_error)
        })
    }
}

fn io_error<E: Display>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err.to_string())
}

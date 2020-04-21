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

use std::{
    io,
    iter::FusedIterator,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    io::{AsyncRead, AsyncWrite},
    sink::SinkExt,
    stream::{self, StreamExt},
};
use futures_codec::{CborCodec, CborCodecError, FramedRead, FramedWrite};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    net::upgrade::{self, Upgraded},
    paths::Paths,
    uri::{Path, Protocol, RadUrn},
};

pub trait CanList {
    type Error;

    fn ls(&self) -> Result<Box<dyn FusedIterator<Item = RadUrn> + Send + Sync>, Self::Error>;
}

#[derive(Clone)]
pub struct Fs {
    paths: Paths,
}

impl Fs {
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    pub fn ls(&self) -> io::Result<impl Iterator<Item = RadUrn> + Send + Sync> {
        let dir = self.paths.projects_dir().read_dir()?;
        let iter = dir.filter_map(|dir_entry| {
            dir_entry.ok().and_then(|dir_entry| {
                git2::Repository::open_bare(dir_entry.path())
                    .ok()
                    .and_then(|_| {
                        dir_entry
                            .file_name()
                            .to_string_lossy()
                            .parse()
                            .ok()
                            .map(|id| RadUrn::new(id, Protocol::Git, Path::with_capacity(0)))
                    })
            })
        });

        Ok(iter)
    }

    pub async fn respond<S>(&self, s: Upgraded<S, upgrade::Ls>) -> Result<(), RespondError>
    where
        S: AsyncWrite + Unpin + Send + Sync,
    {
        respond(self, s).await
    }
}

impl CanList for Fs {
    type Error = io::Error;

    fn ls(&self) -> Result<Box<dyn FusedIterator<Item = RadUrn> + Send + Sync>, Self::Error> {
        let iter = Self::ls(self)?;
        Ok(Box::new(iter.fuse()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    Urn(RadUrn),
    Eof,
}

#[derive(Debug, Error)]
pub enum ResponseError {
    #[error("Invalid payload")]
    InvalidPayload(#[from] serde_cbor::Error),

    #[error(transparent)]
    Io(#[from] io::Error),
}

impl From<CborCodecError> for ResponseError {
    fn from(err: CborCodecError) -> Self {
        match err {
            CborCodecError::Cbor(e) => Self::InvalidPayload(e),
            CborCodecError::Io(e) => Self::Io(e),
        }
    }
}

pub struct ListRemote<S> {
    inner: FramedRead<Upgraded<S, upgrade::Ls>, CborCodec<(), Response>>,
}

impl<S> ListRemote<S>
where
    S: AsyncRead + Unpin + Send + Sync,
{
    pub fn new(s: Upgraded<S, upgrade::Ls>) -> Self {
        Self {
            inner: FramedRead::new(s, CborCodec::new()),
        }
    }
}

impl<S> futures::Stream for ListRemote<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    type Item = Result<Response, ResponseError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.get_mut()
            .inner
            .poll_next_unpin(cx)
            .map_err(|e| e.into())
    }
}

#[derive(Debug, Error)]
pub enum RespondError {
    #[error("Error in `ls`")]
    CanList(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Invalid payload")]
    InvalidPayload(#[from] serde_cbor::Error),

    #[error(transparent)]
    Io(#[from] io::Error),
}

impl From<CborCodecError> for RespondError {
    fn from(err: CborCodecError) -> Self {
        match err {
            CborCodecError::Cbor(e) => Self::InvalidPayload(e),
            CborCodecError::Io(e) => Self::Io(e),
        }
    }
}

pub async fn respond<L, S>(ls: &L, s: Upgraded<S, upgrade::Ls>) -> Result<(), RespondError>
where
    L: CanList,
    L::Error: std::error::Error + Send + Sync + 'static,
    S: AsyncWrite + Unpin + Send + Sync,
{
    let iter = ls.ls().map_err(|e| RespondError::CanList(Box::new(e)))?;
    let mut source = stream::iter(iter).map(Response::Urn).map(Ok);
    let mut sink = FramedWrite::new(s, CborCodec::<Response, ()>::new());
    sink.send_all(&mut source).await?;
    sink.send(Response::Eof).await?;

    Ok(())
}

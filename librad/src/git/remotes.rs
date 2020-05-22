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
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::{paths::Paths, peer::PeerId, uri::RadUrn};

const CONFIG_FILE_NAME: &str = "rad-remotes.config";

pub type Error = git2::Error;

pub struct Remotes {
    config: git2::Config,
    path: PathBuf,
}

unsafe impl Send for Remotes {}

impl Remotes {
    pub fn open(paths: &Paths) -> Result<Self, Error> {
        Self::open_path(paths.git_dir())
    }

    pub(crate) fn open_path(path: &Path) -> Result<Self, Error> {
        let path = path.join(CONFIG_FILE_NAME);
        let config = git2::Config::open(&path)?;
        Ok(Self { config, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn add(&mut self, urn: &RadUrn, remote_peer: &PeerId) -> Result<(), Error> {
        let section = Self::section(urn, remote_peer);

        self.config.set_str(
            &format!("{}.url", &section),
            &urn.as_rad_url_ref(remote_peer).to_string(),
        )?;
        self.config.set_str(
            &format!("{}.fetch", &section),
            &format!(
                "refs/namespaces/{}/refs/*:refs/namespaces/{}/refs/remotes/{}/*",
                &urn.id, &urn.id, remote_peer
            ),
        )?;

        Ok(())
    }

    pub fn remove(&mut self, urn: &RadUrn, remote_peer: &PeerId) -> Result<(), Error> {
        let section = Self::section(urn, remote_peer);

        self.config.remove(&format!("{}.url", &section))?;
        self.config.remove(&format!("{}.fetch", &section))?;

        Ok(())
    }

    pub fn tracked<'a, Context>(&mut self, cx: Context) -> Result<Tracked<'a>, Error>
    where
        Context: Into<Option<&'a RadUrn>>,
    {
        let snapshot = self.config.snapshot()?;
        Ok(Tracked {
            snapshot,
            context: cx.into().map(Cow::Borrowed),
        })
    }

    fn section(urn: &RadUrn, peer: &PeerId) -> String {
        format!("remote.{}/{}", urn.id, peer)
    }
}

pub struct Tracked<'a> {
    snapshot: git2::Config,
    context: Option<Cow<'a, RadUrn>>,
}

impl<'a> Tracked<'a> {
    pub fn iter(&self) -> Result<TrackedPeers, Error> {
        let strip_prefix = self
            .context
            .as_ref()
            .map(|urn| format!("remote.{}/", &urn.id))
            .unwrap_or_else(|| "remote.".to_owned());

        let glob_regex = self
            .context
            .as_ref()
            .map(|urn| format!("^remote.{}/[^.]*.url$", &urn.id))
            .unwrap_or_else(|| "remote.[^.]*.url".to_owned());

        let iter = self.snapshot.entries(Some(&glob_regex))?;
        Ok(TrackedPeers {
            inner: iter,
            strip_prefix,
        })
    }
}

#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TrackedPeers<'a> {
    inner: git2::ConfigEntries<'a>,
    strip_prefix: String,
}

impl<'a> Iterator for TrackedPeers<'a> {
    type Item = Result<PeerId, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        (&self.inner)
            .next()
            .map(|entry| {
                entry.map(|entry| {
                    entry
                        .name()
                        .and_then(|name| name.strip_prefix(&self.strip_prefix))
                        .and_then(|name| name.strip_suffix(".url"))
                        .and_then(|peer| peer.parse().ok())
                })
            })
            .and_then(|res| match res {
                Err(e) => Some(Err(e)),
                Ok(Some(peer)) => Some(Ok(peer)),
                Ok(None) => self.next(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;
    use tempfile::tempdir;

    use crate::{
        hash::Hash,
        keys::SecretKey,
        uri::{self, RadUrn},
    };

    #[test]
    fn test_read_after_write() {
        let tmp = tempdir().unwrap();
        {
            let paths = Paths::from_root(&tmp).unwrap();

            let urn = RadUrn {
                id: Hash::hash(b"abcfdefg"),
                proto: uri::Protocol::Git,
                path: uri::Path::empty(),
            };
            let peer_in = PeerId::from(SecretKey::new());

            let mut remotes = Remotes::open(&paths).unwrap();
            remotes.add(&urn, &peer_in).unwrap();
            let tracked = remotes.tracked(&urn).unwrap();
            let peer_out = tracked.iter().unwrap().next().unwrap().unwrap();

            assert_eq!(peer_in, peer_out)
        }
    }

    #[test]
    fn test_remove() {
        let tmp = tempdir().unwrap();
        {
            let paths = Paths::from_root(&tmp).unwrap();

            let urn = RadUrn {
                id: Hash::hash(b"abcfdefg"),
                proto: uri::Protocol::Git,
                path: uri::Path::empty(),
            };
            let peer_in = PeerId::from(SecretKey::new());

            let mut remotes = Remotes::open(&paths).unwrap();

            remotes.add(&urn, &peer_in).unwrap();
            {
                let tracked = remotes.tracked(&urn).unwrap();
                let peer_out = tracked.iter().unwrap().next().unwrap().unwrap();

                assert_eq!(peer_in, peer_out)
            }

            remotes.remove(&urn, &peer_in).unwrap();
            {
                let tracked = remotes.tracked(&urn).unwrap();
                let peer_out = tracked.iter().unwrap().next();

                assert_eq!(peer_out, None)
            }
        }
    }

    #[test]
    fn test_read_after_write_reopen() {
        let tmp = tempdir().unwrap();
        {
            let paths = Paths::from_root(&tmp).unwrap();

            let urn = RadUrn {
                id: Hash::hash(b"abcfdefg"),
                proto: uri::Protocol::Git,
                path: uri::Path::empty(),
            };
            let peer_in = PeerId::from(SecretKey::new());

            {
                let mut remotes = Remotes::open(&paths).unwrap();
                remotes.add(&urn, &peer_in).unwrap();
            }

            {
                let mut remotes = Remotes::open(&paths).unwrap();
                let tracked = remotes.tracked(&urn).unwrap();
                let peer_out = tracked.iter().unwrap().next().unwrap().unwrap();

                assert_eq!(peer_in, peer_out)
            }
        }
    }

    #[test]
    fn test_remove_reopen() {
        let tmp = tempdir().unwrap();
        {
            let paths = Paths::from_root(&tmp).unwrap();

            let urn = RadUrn {
                id: Hash::hash(b"abcfdefg"),
                proto: uri::Protocol::Git,
                path: uri::Path::empty(),
            };
            let peer_in = PeerId::from(SecretKey::new());

            {
                let mut remotes = Remotes::open(&paths).unwrap();

                remotes.add(&urn, &peer_in).unwrap();
                let tracked = remotes.tracked(&urn).unwrap();
                let peer_out = tracked.iter().unwrap().next().unwrap().unwrap();

                assert_eq!(peer_in, peer_out)
            }

            {
                let mut remotes = Remotes::open(&paths).unwrap();
                remotes.remove(&urn, &peer_in).unwrap();
                let tracked = remotes.tracked(&urn).unwrap();
                let peer_out = tracked.iter().unwrap().next();

                assert_eq!(peer_out, None)
            }
        }
    }

    #[test]
    fn test_concurrent_write() {
        let tmp = tempdir().unwrap();
        {
            let paths = Paths::from_root(&tmp).unwrap();
            let urn = RadUrn {
                id: Hash::hash(b"abcfdefg"),
                proto: uri::Protocol::Git,
                path: uri::Path::empty(),
            };

            // At least one write should've succeeded
            let succeeded = (0..2)
                .map(|_| {
                    track_concurrent(paths.clone(), urn.clone(), PeerId::from(SecretKey::new()))
                })
                .map(|t| t.join())
                .any(|res| res.is_ok());
            assert!(succeeded);
        }
    }

    fn track_concurrent(
        paths: Paths,
        urn: RadUrn,
        peer: PeerId,
    ) -> thread::JoinHandle<Result<(), Error>> {
        thread::spawn(move || {
            let mut remotes = Remotes::open(&paths)?;
            remotes.add(&urn, &peer)
        })
    }
}

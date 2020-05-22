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

use std::sync::{Arc, Mutex, MutexGuard};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

use crate::{
    git::{
        ext::{is_not_found_err, Git2ErrorExt, References},
        remotes::{Remotes, Tracked},
        repo::{self, Repo},
        types::Reference,
    },
    keys::SecretKey,
    meta::entity::{
        data::{EntityBuilder, EntityData},
        Entity,
    },
    paths::Paths,
    peer::PeerId,
    uri::{RadUrl, RadUrn},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Blob {0} not found")]
    NoSuchBlob(String),

    #[error("Branch {0} not found")]
    NoSuchBranch(String),

    #[error(transparent)]
    Git(#[from] git2::Error),
}

#[derive(Clone)]
pub struct Storage {
    backend: Arc<Mutex<git2::Repository>>,
    remotes: Arc<Mutex<Remotes>>,
    pub(crate) key: SecretKey,
}

impl Storage {
    pub fn open(paths: &Paths, key: SecretKey) -> Result<Self, Error> {
        let repo = git2::Repository::open_bare(paths.git_dir())?;
        Self::from_repo(paths, key, repo)
    }

    pub fn init(paths: &Paths, key: SecretKey) -> Result<Self, Error> {
        let repo = git2::Repository::init_opts(
            paths.git_dir(),
            git2::RepositoryInitOptions::new()
                .bare(true)
                .no_reinit(true)
                .external_template(false),
        )?;
        Self::from_repo(paths, key, repo)
    }

    fn from_repo(paths: &Paths, key: SecretKey, repo: git2::Repository) -> Result<Self, Error> {
        let remotes = Remotes::open(paths)?;
        {
            let mut config = repo.config()?;
            config.set_str("user.name", "radicle")?;
            config.set_str("user.email", &format!("radicle@{}", PeerId::from(&key)))?;
            config.set_str("include.path", remotes.path().to_str().unwrap())?;
        }

        Ok(Self {
            backend: Arc::new(Mutex::new(repo)),
            remotes: Arc::new(Mutex::new(remotes)),
            key,
        })
    }

    pub fn create_repo<T>(self, meta: &Entity<T>) -> Result<Repo, repo::Error>
    where
        T: Serialize + DeserializeOwned + Clone + Default,
        EntityData<T>: EntityBuilder,
    {
        Repo::create(self, meta)
    }

    pub fn open_repo(self, urn: RadUrn) -> Result<Repo, repo::Error> {
        Repo::open(self, urn)
    }

    pub fn clone_repo<T>(self, url: RadUrl) -> Result<Repo, repo::Error>
    where
        T: Serialize + DeserializeOwned + Clone + Default,
        EntityData<T>: EntityBuilder,
    {
        Repo::clone(self, url)
    }

    // Utils

    pub(super) fn lock(&self) -> MutexGuard<git2::Repository> {
        self.backend.lock().unwrap()
    }

    pub(crate) fn has_commit(&self, urn: &RadUrn, oid: git2::Oid) -> Result<bool, Error> {
        let span = tracing::warn_span!("Storage::has_commit", urn = %urn, oid = %oid);
        let _guard = span.enter();

        if oid.is_zero() {
            return Ok(false);
        }

        let git = self.lock();
        let commit = git.find_commit(oid);
        match commit {
            Err(e) if is_not_found_err(&e) => {
                tracing::warn!("commit not found");
                Ok(false)
            },
            Ok(commit) => {
                let namespace = &urn.id;
                let branch = urn.path.deref_or_default();
                let branch = branch.strip_prefix("refs/").unwrap_or(branch);

                let refs = References::from_globs(
                    &git,
                    &[format!("refs/namespaces/{}/refs/{}", namespace, branch)],
                )?;

                for (_, oid) in refs.peeled() {
                    if oid == commit.id() || git.graph_descendant_of(oid, commit.id())? {
                        return Ok(true);
                    }
                }

                Ok(false)
            },
            Err(e) => Err(e.into()),
        }
    }

    pub(crate) fn has_ref(&self, reference: &Reference) -> Result<bool, Error> {
        self.lock()
            .find_reference(&reference.to_string())
            .map(|_| true)
            .map_not_found(|| Ok(false))
    }

    pub(crate) fn has_urn(&self, urn: &RadUrn) -> Result<bool, Error> {
        let namespace = &urn.id;
        let branch = urn.path.deref_or_default();
        let branch = branch.strip_prefix("refs/").unwrap_or(branch);
        self.lock()
            .find_reference(&format!("refs/namespaces/{}/refs/{}", namespace, branch))
            .map(|_| true)
            .map_not_found(|| Ok(false))
    }

    pub(crate) fn track(&self, urn: &RadUrn, peer: &PeerId) -> Result<(), Error> {
        self.remotes
            .lock()
            .unwrap()
            .add(urn, peer)
            .map_err(|e| e.into())
    }

    pub(crate) fn untrack(&self, urn: &RadUrn, peer: &PeerId) -> Result<(), Error> {
        self.remotes
            .lock()
            .unwrap()
            .remove(urn, peer)
            .map_err(|e| e.into())
    }

    pub(crate) fn tracked<'urn>(&self, urn: &'urn RadUrn) -> Result<Tracked<'urn>, Error> {
        let mut remotes = self.remotes.lock().unwrap();
        let tracked = remotes.tracked(Some(urn))?;
        Ok(tracked)
    }
}

pub enum WithBlob<'a> {
    Tip {
        reference: &'a Reference,
        file_name: &'a str,
    },
    Init {
        reference: &'a Reference,
        file_name: &'a str,
    },
}

impl<'a> WithBlob<'a> {
    pub fn get(self, git: &'a git2::Repository) -> Result<git2::Blob<'a>, Error> {
        match self {
            Self::Tip {
                reference,
                file_name,
            } => {
                let ref_name = reference.to_string();
                let branch = git
                    .find_reference(&ref_name)
                    .map_not_found(|| Err(Error::NoSuchBranch(ref_name)))?;
                let tree = branch.peel_to_tree()?;
                blob(git, tree, file_name)
            },

            Self::Init {
                reference,
                file_name,
            } => {
                let mut revwalk = git.revwalk()?;
                let mut sort = git2::Sort::TOPOLOGICAL;
                sort.insert(git2::Sort::REVERSE);
                revwalk.set_sorting(sort)?;
                revwalk.simplify_first_parent()?;
                revwalk.push_ref(&reference.to_string())?;

                match revwalk.next() {
                    None => Err(Error::NoSuchBlob(file_name.to_owned())),
                    Some(oid) => {
                        let oid = oid?;
                        let tree = git.find_commit(oid)?.tree()?;
                        blob(git, tree, file_name)
                    },
                }
            },
        }
    }
}

fn blob<'a>(
    repo: &'a git2::Repository,
    tree: git2::Tree<'a>,
    file_name: &'a str,
) -> Result<git2::Blob<'a>, Error> {
    let entry = tree
        .get_name(file_name)
        .ok_or_else(|| Error::NoSuchBlob(file_name.to_owned()))?;
    let bob = entry.to_object(repo)?.peel_to_blob()?;

    Ok(bob)
}

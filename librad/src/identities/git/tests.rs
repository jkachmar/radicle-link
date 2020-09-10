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

use std::io;

use crate::keys::SecretKey;

use librad_test::tempdir::WithTmpDir;
use pretty_assertions::assert_eq;

use super::*;

lazy_static! {
    // Keys
    static ref CHANTAL_SECRET: SecretKey = SecretKey::new();
    static ref DYLAN_SECRET: SecretKey = SecretKey::new();

    // Payloads
    static ref CHANTAL_PAYLOAD: UserPayload = payload::User {
        name: "chantal".into()
    }
    .into();
    static ref DYLAN_PAYLOAD: UserPayload = payload::User {
        name: "dylan".into()
    }
    .into();
    static ref CRUCIAL_PROJECT_PAYLOAD: ProjectPayload = payload::Project {
        name: "haskell-emoji".into(),
        description: Some("The most important software package in the world".into()),
        default_branch: Some("\u{1F32F}".into()),
    }
    .into();
}

type TmpRepo = WithTmpDir<git2::Repository>;

fn repo() -> Result<TmpRepo, Box<dyn std::error::Error>> {
    Ok(WithTmpDir::new(|path| {
        git2::Repository::init(path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    })?)
}

fn chantal(handle: &Git<'_, User>) -> Result<User, Box<dyn std::error::Error>> {
    Ok(handle.create(
        CHANTAL_PAYLOAD.clone(),
        Some(CHANTAL_SECRET.public()).into_iter().collect(),
        &*CHANTAL_SECRET,
    )?)
}

fn dylan(handle: &Git<'_, User>) -> Result<User, Box<dyn std::error::Error>> {
    Ok(handle.create(
        DYLAN_PAYLOAD.clone(),
        Some(DYLAN_SECRET.public()).into_iter().collect(),
        &*DYLAN_SECRET,
    )?)
}

#[test]
fn create_user() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let handle = Git::<User>::new(&repo);

    let chantal = chantal(&handle)?;
    let real_chantal = handle.verify(*chantal.content_id)?.into_inner();

    assert_eq!(chantal, real_chantal);

    Ok(())
}

#[test]
fn update_user() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let handle = Git::<User>::new(&repo);

    let chantal = chantal(&handle)?;
    let chantal_revision = chantal.revision;

    let chantal_and_dylan: delegation::Direct =
        vec![CHANTAL_SECRET.public(), DYLAN_SECRET.public()]
            .into_iter()
            .collect();

    let chantal2 = handle.update(
        Verifying::from(chantal).signed()?,
        None,
        Some(chantal_and_dylan),
        &*CHANTAL_SECRET,
    )?;

    // chantal2 doesn't reach quorum, so verify should yield the initial revision
    assert_eq!(
        handle.verify(*chantal2.content_id)?.revision,
        chantal_revision
    );

    // Dylan can help to reach the quorum, tho
    let dylan = handle.create_from(Verifying::from(chantal2).signed()?, &*DYLAN_SECRET)?;
    let real_dylan = handle.verify(*dylan.content_id)?.into_inner();

    assert_eq!(dylan, real_dylan);

    Ok(())
}

#[test]
fn create_project() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let handle = Git::<Project>::new(&repo);

    let chantal = chantal(&handle.as_user())?;
    let chantal_head = chantal.content_id;
    let delegations = delegation::Indirect::try_from_iter(Some(Right(chantal)))?;

    let hs_emoji = handle.create(
        CRUCIAL_PROJECT_PAYLOAD.clone(),
        delegations,
        &*CHANTAL_SECRET,
    )?;
    let real_hs_emoji = handle
        .verify::<_, !>(*hs_emoji.content_id, |_| Ok(*chantal_head))?
        .into_inner();

    assert_eq!(hs_emoji, real_hs_emoji);

    Ok(())
}

#[test]
fn update_project() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let handle = Git::<Project>::new(&repo);

    let chantal = chantal(&handle.as_user())?;
    let chantal_urn = chantal.urn();
    let chantal_head = chantal.content_id;

    let dylan = dylan(&handle.as_user())?;
    let dylan_urn = dylan.urn();
    let dylan_head = dylan.content_id;

    let resolve_latest = |urn| {
        if urn == chantal_urn {
            Ok(*chantal_head)
        } else if urn == dylan_urn {
            Ok(*dylan_head)
        } else {
            unreachable!()
        }
    };

    let hs_emoji = handle.create(
        CRUCIAL_PROJECT_PAYLOAD.clone(),
        IndirectDelegation::try_from_iter(Some(Right(chantal.clone())))?,
        &*CHANTAL_SECRET,
    )?;
    let hs_emoji_revision = hs_emoji.revision;

    let hs_emoji2 = handle.update(
        Verifying::from(hs_emoji).signed()?,
        None,
        IndirectDelegation::try_from_iter(vec![Right(chantal), Right(dylan)])?,
        &*CHANTAL_SECRET,
    )?;

    // hs_emoji2 doesn't reach quorum, so verify should yield the initial revision
    assert_eq!(
        handle
            .verify::<_, !>(*hs_emoji2.content_id, resolve_latest)?
            .revision,
        hs_emoji_revision
    );

    // So dylan, approve s'il vous plait
    let dylans_emoji = handle.create_from(Verifying::from(hs_emoji2).signed()?, &*DYLAN_SECRET)?;
    assert_eq!(
        handle
            .verify(*dylans_emoji.content_id, resolve_latest)?
            .into_inner(),
        dylans_emoji
    );

    Ok(())
}

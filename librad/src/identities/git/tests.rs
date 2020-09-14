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
    static ref EVE_SECRET: SecretKey = SecretKey::new();

    // Payloads
    static ref CHANTAL_PAYLOAD: UserPayload = payload::User {
        name: "chantal".into()
    }
    .into();
    static ref DYLAN_PAYLOAD: UserPayload = payload::User {
        name: "dylan".into()
    }
    .into();
    static ref EVE_PAYLOAD: UserPayload = payload::User {
        name: "eve".into()
    }
    .into();
    static ref CRUCIAL_PROJECT_PAYLOAD: ProjectPayload = payload::Project {
        name: "haskell-emoji".into(),
        description: Some("The most important software package in the world".into()),
        default_branch: Some("\u{1F32F}".into()),
    }
    .into();

    // Delegations
    static ref CHANTAL_DIRECT: delegation::Direct =
        Some(CHANTAL_SECRET.public()).into_iter().collect();
    static ref DYLAN_DIRECT: delegation::Direct =
        Some(DYLAN_SECRET.public()).into_iter().collect();
    static ref EVE_DIRECT: delegation::Direct =
        Some(EVE_SECRET.public()).into_iter().collect();
    static ref CHANTAL_AND_DYLAN_DIRECT: delegation::Direct =
        vec![CHANTAL_SECRET.public(), DYLAN_SECRET.public()].into_iter().collect();
    static ref CHANTAL_DYLAN_AND_EVE_DIRECT: delegation::Direct =
        vec![CHANTAL_SECRET.public(), DYLAN_SECRET.public(), EVE_SECRET.public()]
            .into_iter().collect();
}

type TmpRepo = WithTmpDir<git2::Repository>;

fn repo() -> Result<TmpRepo, Box<dyn std::error::Error>> {
    Ok(WithTmpDir::new(|path| {
        let setup = || {
            let repo = git2::Repository::init(path)?;

            // We need to set user info to _something_, but that doesn't have to
            // be valid, as we're using a shared repo with many keys
            let mut config = repo.config()?;
            config.set_str("user.name", "shared")?;
            config.set_str("user.email", "not.relevant@for.testing")?;
            Ok(repo)
        };
        setup().map_err(|e: git2::Error| io::Error::new(io::ErrorKind::Other, e))
    })?)
}

fn chantal(handle: &Git<'_, User>) -> Result<User, Box<dyn std::error::Error>> {
    Ok(handle.create(
        CHANTAL_PAYLOAD.clone(),
        CHANTAL_DIRECT.clone(),
        &*CHANTAL_SECRET,
    )?)
}

fn dylan(handle: &Git<'_, User>) -> Result<User, Box<dyn std::error::Error>> {
    Ok(handle.create(DYLAN_PAYLOAD.clone(), DYLAN_DIRECT.clone(), &*DYLAN_SECRET)?)
}

#[test]
fn create_user() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let handle = Git::<User>::new(&repo);

    let chantal = chantal(&handle)?;
    assert_eq!(handle.verify(*chantal.content_id)?.into_inner(), chantal);

    Ok(())
}

#[test]
fn update_user() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let handle = Git::<User>::new(&repo);

    let chantal = chantal(&handle)?;

    let chantal2 = handle.update(
        Verifying::from(chantal).signed()?,
        None,
        CHANTAL_AND_DYLAN_DIRECT.clone(),
        &*CHANTAL_SECRET,
    )?;
    // No quorum
    assert_matches!(
        Verifying::from(handle.get(*chantal2.content_id)?)
            .signed()?
            .quorum(),
        Err(VerificationError::Quorum)
    );

    // Dylan can help tho
    let dylan = handle.create_from(Verifying::from(chantal2).signed()?, &*DYLAN_SECRET)?;
    assert_eq!(handle.verify(*dylan.content_id)?.into_inner(), dylan);

    Ok(())
}

#[test]
fn user_revoke_delegation() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let handle = Git::<User>::new(&repo);

    // Kickstart
    let chantal = chantal(&handle)?;

    // <3 Dylan
    let chantal2 = handle.update(
        Verifying::from(chantal).signed()?,
        None,
        CHANTAL_DYLAN_AND_EVE_DIRECT.clone(),
        &*CHANTAL_SECRET,
    )?;

    // Approve mutually
    let dylan = handle.create_from(Verifying::from(chantal2.clone()).signed()?, &*DYLAN_SECRET)?;
    let eve = handle.create_from(Verifying::from(chantal2.clone()).signed()?, &*EVE_SECRET)?;
    let chantal3 = {
        let merge_dylan = handle.update_from(
            Verifying::from(chantal2).signed()?,
            Verifying::from(dylan.clone()).signed()?,
            &*CHANTAL_SECRET,
        )?;
        handle.update_from(
            Verifying::from(merge_dylan).signed()?,
            Verifying::from(eve).signed()?,
            &*CHANTAL_SECRET,
        )
    }?;

    assert_eq!(handle.verify(*chantal3.content_id)?.into_inner(), chantal3);

    // Kick out eve
    let chantal4 = handle.update(
        Verifying::from(chantal3).signed()?,
        None,
        CHANTAL_AND_DYLAN_DIRECT.clone(),
        &*CHANTAL_SECRET,
    )?;
    // That doesn't reach quorum, so we should get chantal3 back
    assert_matches!(
        Verifying::from(handle.get(*chantal4.content_id)?)
            .signed()?
            .quorum(),
        Err(VerificationError::Quorum)
    );

    // Dylan doesn't like Eve, either
    let dylan2 = handle.update_from(
        Verifying::from(dylan).signed()?,
        Verifying::from(chantal4).signed()?,
        &*DYLAN_SECRET,
    )?;
    // So that checks out
    assert_eq!(handle.verify(*dylan2.content_id)?.into_inner(), dylan2);

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
    assert_eq!(
        handle
            .verify::<_, !>(*hs_emoji.content_id, |_| Ok(*chantal_head))?
            .into_inner(),
        hs_emoji
    );

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
    assert_eq!(
        handle
            .verify::<_, !>(*hs_emoji.content_id, resolve_latest)?
            .into_inner(),
        hs_emoji
    );
    let hs_emoji2 = handle.update(
        Verifying::from(hs_emoji).signed()?,
        None,
        IndirectDelegation::try_from_iter(vec![Right(chantal), Right(dylan)])?,
        &*CHANTAL_SECRET,
    )?;
    // No quorum yet
    assert_matches!(
        Verifying::from(handle.get(*hs_emoji2.content_id)?)
            .signed()?
            .quorum(),
        Err(VerificationError::Quorum)
    );

    // So dylan, approve s'il vous plait
    let dylans_emoji = handle.create_from(Verifying::from(hs_emoji2).signed()?, &*DYLAN_SECRET)?;
    assert_eq!(
        handle
            .verify::<_, !>(*dylans_emoji.content_id, resolve_latest)?
            .into_inner(),
        dylans_emoji
    );

    Ok(())
}

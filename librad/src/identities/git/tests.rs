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
    static ref CHANTAL_SECRET: SecretKey = SecretKey::new();
    static ref DYLAN_SECRET: SecretKey = SecretKey::new();
}

type TmpRepo = WithTmpDir<git2::Repository>;

fn repo() -> TmpRepo {
    WithTmpDir::new(|path| {
        git2::Repository::init(path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    })
    .unwrap()
}

fn chantal(handle: &Git<'_, User>) -> Result<User, Box<dyn std::error::Error>> {
    Ok(handle.create(
        payload::User {
            name: "chantal".into(),
        }
        .into(),
        Some(CHANTAL_SECRET.public()).into_iter().collect(),
        &*CHANTAL_SECRET,
    )?)
}

fn dylan(handle: &Git<'_, User>) -> Result<User, Box<dyn std::error::Error>> {
    Ok(handle.create(
        payload::User {
            name: "dylan".into(),
        }
        .into(),
        Some(DYLAN_SECRET.public()).into_iter().collect(),
        &*DYLAN_SECRET,
    )?)
}

#[test]
fn create_user() {
    let repo = repo();
    let handle = Git::<User>::new(&repo);

    let chantal = chantal(&handle).unwrap();
    let real_chantal = handle.verify(*chantal.content_id).unwrap().into_inner();

    assert_eq!(chantal, real_chantal)
}

#[test]
fn update_user() {
    let repo = repo();
    let handle = Git::<User>::new(&repo);

    let chantal = chantal(&handle).unwrap();
    let chantal_revision = chantal.revision;

    let chantal_and_dylan: delegation::Direct =
        vec![CHANTAL_SECRET.public(), DYLAN_SECRET.public()]
            .into_iter()
            .collect();

    let chantal2 = handle
        .update(
            Verifying::from(chantal).signed().unwrap(),
            None,
            Some(chantal_and_dylan),
            &*CHANTAL_SECRET,
        )
        .unwrap();

    // chantal2 doesn't reach quorum, so verify should yield the initial revision
    assert_eq!(
        handle.verify(*chantal2.content_id).unwrap().revision,
        chantal_revision
    );

    // Dylan can help to reach the quorum, tho
    let dylan = handle
        .create_from(Verifying::from(chantal2).signed().unwrap(), &*DYLAN_SECRET)
        .unwrap();
    let real_dylan = handle.verify(*dylan.content_id).unwrap().into_inner();

    assert_eq!(dylan, real_dylan)
}

#[test]
fn create_project() {
    let repo = repo();
    let handle = Git::<Project>::new(&repo);

    let chantal = chantal(&handle.as_user()).unwrap();
    let chantal_head = chantal.content_id;
    let delegations = delegation::Indirect::try_from_iter(Some(Right(chantal))).unwrap();

    let hs_emoji = handle
        .create(
            payload::Project {
                name: "haskell-emoji".into(),
                description: Some("The most important software package in the world".into()),
                default_branch: Some("\u{1F32F}".into()),
            }
            .into(),
            delegations,
            &*CHANTAL_SECRET,
        )
        .unwrap();
    let real_hs_emoji = handle
        .verify::<_, !>(*hs_emoji.content_id, |_| Ok(*chantal_head))
        .unwrap()
        .into_inner();

    assert_eq!(hs_emoji, real_hs_emoji)
}

#[test]
fn update_project() {
    let repo = repo();
    let handle = Git::<Project>::new(&repo);

    let chantal = chantal(&handle.as_user()).unwrap();
    let chantal_urn = chantal.urn();
    let chantal_head = chantal.content_id;

    let dylan = dylan(&handle.as_user()).unwrap();
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

    let hs_emoji = handle
        .create(
            payload::Project {
                name: "haskell-emoji".into(),
                description: Some("The most important software package in the world".into()),
                default_branch: Some("\u{1F32F}".into()),
            }
            .into(),
            IndirectDelegation::try_from_iter(Some(Right(chantal.clone()))).unwrap(),
            &*CHANTAL_SECRET,
        )
        .unwrap();
    let hs_emoji_revision = hs_emoji.revision;

    let hs_emoji2 = handle
        .update(
            Verifying::from(hs_emoji).signed().unwrap(),
            None,
            IndirectDelegation::try_from_iter(vec![Right(chantal), Right(dylan)]).unwrap(),
            &*CHANTAL_SECRET,
        )
        .unwrap();

    // hs_emoji2 doesn't reach quorum, so verify should yield the initial revision
    assert_eq!(
        handle
            .verify::<_, !>(*hs_emoji2.content_id, resolve_latest)
            .unwrap()
            .revision,
        hs_emoji_revision
    );

    // So dylan, approve s'il vous plait
    let dylans_emoji = handle
        .create_from(Verifying::from(hs_emoji2).signed().unwrap(), &*DYLAN_SECRET)
        .unwrap();
    assert_eq!(
        handle
            .verify(*dylans_emoji.content_id, resolve_latest)
            .unwrap()
            .into_inner(),
        dylans_emoji
    )
}

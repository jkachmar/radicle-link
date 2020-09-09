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

fn mk_chantal(handle: &Git<'_, User>) -> Result<User, Box<dyn std::error::Error>> {
    Ok(handle.create(
        UserPayload::new(payload::User {
            name: "chantal".into(),
        }),
        Some(CHANTAL_SECRET.public()).into_iter().collect(),
        &*CHANTAL_SECRET,
    )?)
}

#[test]
fn create_user() {
    let repo = repo();
    let handle = Git::<User>::new(&repo);

    let chantal = mk_chantal(&handle).unwrap();
    let real_chantal = handle.verify(*chantal.content_id).unwrap().into_inner();

    assert_eq!(chantal, real_chantal)
}

#[test]
fn update_user() {
    let repo = repo();
    let handle = Git::<User>::new(&repo);

    let chantal = mk_chantal(&handle).unwrap();
    let chantal_revision = chantal.revision;

    let chantal_and_dylan: delegation::Direct =
        vec![CHANTAL_SECRET.public(), DYLAN_SECRET.public()]
            .into_iter()
            .collect();

    let chantal2 = handle
        .update(
            generic::Verifying::from(chantal).signed().unwrap(),
            None,
            Some(chantal_and_dylan),
            &*CHANTAL_SECRET,
        )
        .unwrap();

    let chantal2_verified = handle.verify(*chantal2.content_id).unwrap();
    // chantal2 doesn't reach quorum, so verify should yield the initial revision
    assert_eq!(chantal_revision, chantal2_verified.revision);

    // Dylan can help to reach the quorum, tho
    let dylan = handle
        .create_from(
            generic::Verifying::from(chantal2).signed().unwrap(),
            &*DYLAN_SECRET,
        )
        .unwrap();
    let real_dylan = handle.verify(*dylan.content_id).unwrap().into_inner();

    assert_eq!(dylan, real_dylan)
}

#[test]
fn create_project() {
    let repo = repo();
    let handle = Git::<User>::new(&repo);

    let chantal = mk_chantal(&handle).unwrap();
    let chantal_head = chantal.content_id;
    let delegations = delegation::Indirect::try_from_iter(Some(Right(chantal))).unwrap();

    let hs_emoji = handle
        .as_project()
        .create(
            ProjectPayload::new(payload::Project {
                name: "haskell-emoji".into(),
                description: Some("The most important software package in the world".into()),
                default_branch: Some("\u{1F32F}".into()),
            }),
            delegations,
            &*CHANTAL_SECRET,
        )
        .unwrap();
    let real_hs_emoji = handle
        .as_project()
        .verify::<_, !>(*hs_emoji.content_id, |_| Ok(*chantal_head))
        .unwrap()
        .into_inner();

    assert_eq!(hs_emoji, real_hs_emoji)
}

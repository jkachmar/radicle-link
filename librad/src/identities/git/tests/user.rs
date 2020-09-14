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

use anyhow::anyhow;
use sodiumoxide::crypto::sign::ed25519::Seed;

use super::*;
use crate::keys::SecretKey;

lazy_static! {
    static ref DESKTOP: SecretKey = SecretKey::from_seed(&Seed([
        143, 47, 243, 180, 88, 210, 28, 210, 95, 46, 192, 56, 51, 195, 64, 222, 206, 58, 197, 225,
        9, 65, 102, 201, 120, 103, 253, 204, 96, 186, 112, 5
    ]));
    static ref LAPTOP: SecretKey = SecretKey::from_seed(&Seed([
        30, 242, 189, 126, 37, 140, 20, 42, 81, 142, 241, 147, 125, 104, 39, 52, 116, 251, 203,
        128, 121, 28, 90, 176, 119, 91, 59, 205, 180, 97, 134, 185
    ]));
    static ref PALMTOP: SecretKey = SecretKey::from_seed(&Seed([
        175, 193, 135, 176, 191, 147, 253, 103, 100, 182, 201, 116, 62, 99, 240, 24, 224, 48, 170,
        34, 124, 181, 132, 3, 192, 82, 110, 111, 22, 22, 113, 200
    ]));
}

#[derive(Clone)]
struct Device<'a> {
    key: &'a SecretKey,
    git: Git<'a, User>,
    cur: User,
}

impl<'a> Device<'a> {
    fn new(key: &'a SecretKey, git: Git<'a, User>) -> anyhow::Result<Self> {
        let cur = git.create(
            payload::User {
                name: "dylan".into(),
            }
            .into(),
            Some(key.public()).into_iter().collect(),
            key,
        )?;

        Ok(Self { key, git, cur })
    }

    fn create_from(key: &'a SecretKey, other: &Device<'a>) -> anyhow::Result<Self> {
        let cur = other
            .git
            .create_from(Verifying::from(other.cur.clone()).signed()?, key)?;
        Ok(Self {
            key,
            cur,
            git: Git::new(other.git.repo),
        })
    }

    fn update(self, delegations: impl Into<Option<delegation::Direct>>) -> anyhow::Result<Self> {
        let cur = self.git.update(
            Verifying::from(self.cur).signed()?,
            None,
            delegations,
            self.key,
        )?;

        Ok(Self { cur, ..self })
    }

    fn update_from(self, other: &Device<'a>) -> anyhow::Result<Self> {
        let cur = self.git.update_from(
            Verifying::from(self.cur).signed()?,
            Verifying::from(other.cur.clone()).signed()?,
            self.key,
        )?;

        Ok(Self { cur, ..self })
    }

    fn verify(&self) -> Result<VerifiedUser, error::VerifyUser> {
        Ok(self.git.verify(*self.cur.content_id)?)
    }

    fn assert_verifies(&self) -> anyhow::Result<()> {
        let verified = self.git.verify(*self.cur.content_id)?.into_inner();
        anyhow::ensure!(
            verified == self.cur,
            anyhow!(
                "verified head `{}` is not current head `{}`",
                verified.content_id,
                self.cur.content_id
            )
        );
        Ok(())
    }

    fn assert_no_quorum(&self) -> anyhow::Result<()> {
        let quorum = Verifying::from(self.cur.clone()).signed()?.quorum();
        anyhow::ensure!(
            matches!(quorum, Err(VerificationError::Quorum)),
            anyhow!(
                "expected {} not not reach quorum, instead this happened: {:?}",
                self.cur.content_id,
                quorum
            )
        );
        Ok(())
    }
}

#[test]
fn create() -> anyhow::Result<()> {
    let repo = repo()?;
    {
        Device::new(&*DESKTOP, Git::new(&repo))?.assert_verifies()
    }
}

#[test]
fn update() -> anyhow::Result<()> {
    let repo = repo()?;
    {
        let desktop = Device::new(&*DESKTOP, Git::new(&repo))?.update(Some(
            vec![DESKTOP.public(), LAPTOP.public()]
                .into_iter()
                .collect(),
        ))?;
        desktop.assert_no_quorum()?;

        // Gotta confirm from laptop
        let laptop = Device::create_from(&*LAPTOP, &desktop)?;
        laptop.assert_verifies()?;

        // Now that should be a fast-forward on the desktop
        desktop.update_from(&laptop)?.assert_verifies()
    }
}

#[test]
fn revoke_a_deux() -> anyhow::Result<()> {
    let repo = repo()?;
    {
        let desktop = Device::new(&*DESKTOP, Git::new(&repo))?.update(Some(
            vec![DESKTOP.public(), LAPTOP.public()]
                .into_iter()
                .collect(),
        ))?;

        // Kick out desktop
        let laptop = Device::create_from(&*LAPTOP, &desktop)?;
        let laptop_revokes_desktop = laptop
            .clone()
            .update(Some(Some(LAPTOP.public()).into_iter().collect()))?;
        // Cannot do that unilaterally -- laptop is now invalid
        assert_matches!(
            laptop_revokes_desktop.verify(),
            Err(error::VerifyUser::Verification(
                VerificationError::ParentQuorum
            ))
        );

        // Ack confirmation, and then revocation
        let desktop = desktop
            .update_from(&laptop)?
            .update_from(&laptop_revokes_desktop)?;
        desktop.assert_verifies()?;

        // Now laptop turns valid again
        let laptop = laptop.update_from(&desktop)?;
        laptop.assert_verifies()
    }
}

#[test]
fn revoke_a_trois() -> anyhow::Result<()> {
    let repo = repo()?;
    {
        let desktop = Device::new(&*DESKTOP, Git::new(&repo))?.update(Some(
            vec![DESKTOP.public(), LAPTOP.public(), PALMTOP.public()]
                .into_iter()
                .collect(),
        ))?;

        // We don't have to ask palmtop for it to be added
        let laptop = Device::create_from(&*LAPTOP, &desktop)?;
        laptop.assert_verifies()?;

        let desktop = desktop.update_from(&laptop)?;
        desktop.assert_verifies()?;

        // And we don't have to ask it to be removed either
        let desktop = desktop.update(Some(
            vec![DESKTOP.public(), LAPTOP.public()]
                .into_iter()
                .collect(),
        ))?;

        let laptop = laptop.update_from(&desktop)?;
        laptop.assert_verifies()?;
        let desktop = desktop.update_from(&laptop)?;
        desktop.assert_verifies()
    }
}

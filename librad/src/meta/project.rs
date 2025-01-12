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

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    meta::{
        common::{Label, Url},
        entity::{
            data::{EntityData, EntityInfoExt, EntityKind},
            Draft,
            Entity,
            Error,
        },
    },
    uri::RadUrn,
};

pub const DEFAULT_BRANCH: &str = "master";

pub fn default_branch() -> String {
    DEFAULT_BRANCH.into()
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
pub struct ProjectInfo {
    // Marker so `EntityInfo` can deserialize correctly
    project: (),

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default = "default_branch")]
    pub default_branch: String,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub rel: Vec<Relation>,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        Self {
            project: (),
            description: None,
            default_branch: DEFAULT_BRANCH.into(),
            rel: vec![],
        }
    }
}

impl EntityInfoExt for ProjectInfo {
    fn kind(&self) -> EntityKind {
        EntityKind::Project
    }

    fn check_invariants<T>(&self, data: &EntityData<T>) -> Result<(), Error> {
        if data.certifiers.is_empty() {
            return Err(Error::InvalidData("Missing certifier".to_owned()));
        }
        Ok(())
    }
}

impl ProjectInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_rel(&mut self, rel: Relation) {
        self.rel.push(rel)
    }

    pub fn add_rels(&mut self, rels: &[Relation]) {
        self.rel.extend_from_slice(rels)
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
pub enum Relation {
    Tag(Label),
    Label(Label, String),
    Url(Label, Url),
    Path(Label, PathBuf),
}

pub type ProjectData = EntityData<ProjectInfo>;

impl ProjectData {
    pub fn set_optional_description(mut self, description: Option<String>) -> Self {
        self.info.description = description;
        self
    }

    pub fn set_description(mut self, description: String) -> Self {
        self.info.description = Some(description);
        self
    }

    pub fn clear_description(mut self) -> Self {
        self.info.description = None;
        self
    }

    pub fn set_default_branch(mut self, default_branch: String) -> Self {
        self.info.default_branch = default_branch;
        self
    }

    pub fn add_rel(mut self, rel: Relation) -> Self {
        self.info.add_rel(rel);
        self
    }

    pub fn add_rels(mut self, rels: &[Relation]) -> Self {
        self.info.add_rels(rels);
        self
    }

    pub fn add_maintainer(self, maintainer: RadUrn) -> Self {
        self.add_certifier(maintainer)
    }

    pub fn add_maintainers(self, maintainers: &[RadUrn]) -> Self {
        self.add_certifiers(maintainers.iter().cloned())
    }
}

pub type Project<ST> = Entity<ProjectInfo, ST>;

impl<ST> Project<ST>
where
    ST: Clone,
{
    pub fn maintainers(&self) -> &std::collections::HashSet<RadUrn> {
        self.certifiers()
    }

    pub fn description(&self) -> &Option<String> {
        &self.info().description
    }

    pub fn default_branch(&self) -> &str {
        &self.info().default_branch
    }

    pub fn rels(&self) -> &[Relation] {
        &self.info().rel
    }

    pub fn create(name: String, owner: RadUrn) -> Result<Project<Draft>, Error> {
        ProjectData::default()
            .set_name(name)
            .set_revision(1)
            .add_certifier(owner)
            .build()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::meta::{entity::GenericDraftEntity, entity_test::EMPTY_URI};

    use proptest::prelude::*;

    #[test]
    fn test_project_serde() {
        let proj = Project::<Draft>::create("foo".to_owned(), EMPTY_URI.clone()).unwrap();
        let proj_ser = serde_json::to_string(&proj).unwrap();
        let proj_de = serde_json::from_str(&proj_ser).unwrap();
        assert_eq!(proj, proj_de)
    }

    fn gen_project() -> impl Strategy<Value = Project<Draft>> {
        (
            ".*",
            proptest::option::of(".*"),
            ".*",
            proptest::collection::vec(Just(EMPTY_URI.to_owned()), 1..32),
            proptest::collection::vec(gen_relation(), 0..16),
        )
            .prop_map(|(name, description, branch, maintainers, rels)| {
                ProjectData::default()
                    .set_revision(1)
                    .set_name(name)
                    .set_optional_description(description)
                    .set_default_branch(branch)
                    .add_maintainers(&maintainers)
                    .add_rels(&rels)
                    .build()
                    .unwrap()
            })
    }

    fn gen_relation() -> impl Strategy<Value = Relation> {
        prop_oneof![
            ".*".prop_map(Relation::Tag),
            (".*", ".*").prop_map(|(l, v)| Relation::Label(l, v)),
            ".*".prop_map(|l| Relation::Url(l, Url::parse("https://acme.com/x/y").unwrap())),
            (".*", prop::collection::vec(".*", 1..32))
                .prop_map(|(l, xs)| Relation::Path(l, xs.iter().collect())),
        ]
    }

    proptest! {
        #[test]
        fn prop_relation_serde(rel in gen_relation()) {
            let rel_de = serde_json::to_string(&rel)
                .and_then(|ser| serde_json::from_str(&ser))
                .unwrap();
            assert_eq!(rel, rel_de)
        }

        #[test]
        fn prop_project_serde(proj in gen_project()) {
            let ser = serde_json::to_string(&proj).unwrap();
            let proj_de = Project::<Draft>::from_json_str(&ser).unwrap();
            assert_eq!(proj, proj_de);

            println!(" --- project gen deserialize");
            let generic_de = GenericDraftEntity::from_json_str(&ser).unwrap();
            let generic_ser = generic_de.to_json_string().unwrap();
            assert_eq!(ser, generic_ser);
        }
    }
}

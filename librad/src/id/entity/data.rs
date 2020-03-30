use crate::id::Error;
use multihash::{Multihash, Sha2_256};
use olpc_cjson::CanonicalFormatter;
use serde::{de::DeserializeOwned, Deserialize, Serialize, Serializer};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
pub struct EntitySignatureData {
    pub user: Option<String>,
    pub sig: String,
}

fn ordered_set<S>(value: &HashSet<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeSet<String> = value.iter().cloned().collect();
    ordered.serialize(serializer)
}

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct EntityData<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<HashMap<String, EntitySignatureData>>,

    #[serde(
        skip_serializing_if = "HashSet::is_empty",
        serialize_with = "ordered_set",
        default
    )]
    pub keys: HashSet<String>,
    #[serde(
        skip_serializing_if = "HashSet::is_empty",
        serialize_with = "ordered_set",
        default
    )]
    pub certifiers: HashSet<String>,

    pub info: T,
}

impl<T> EntityData<T>
where
    T: Serialize + DeserializeOwned + Clone + Default,
{
    pub fn to_json_writer<W>(&self, writer: W) -> Result<(), Error>
    where
        W: std::io::Write,
    {
        serde_json::to_writer(writer, self).map_err(Error::SerializationFailed)?;
        Ok(())
    }
    pub fn to_json_string(&self) -> Result<String, Error> {
        Ok(serde_json::to_string(self).map_err(Error::SerializationFailed)?)
    }

    pub fn from_json_reader<R>(r: R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        Ok(serde_json::from_reader(r).map_err(Error::SerializationFailed)?)
    }
    pub fn from_json_str(s: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(s).map_err(Error::SerializationFailed)?)
    }

    pub fn canonical_data(&self) -> Result<Vec<u8>, Error> {
        let mut cleaned = EntityData::<T>::default();
        cleaned.name = self.name.to_owned();
        cleaned.revision = self.revision.to_owned();
        cleaned.hash = self.hash.to_owned();
        cleaned.keys = self.keys.to_owned();
        cleaned.certifiers = self.certifiers.to_owned();
        cleaned.info = self.info.to_owned();

        let mut buffer: Vec<u8> = vec![];
        let mut ser =
            serde_json::Serializer::with_formatter(&mut buffer, CanonicalFormatter::new());
        cleaned
            .serialize(&mut ser)
            .map_err(Error::SerializationFailed)?;
        Ok(buffer)
    }

    pub fn compute_hash(&self) -> Result<Multihash, Error> {
        Ok(Sha2_256::digest(&self.canonical_data()?))
    }
}

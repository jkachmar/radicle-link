use crate::id::entity::Error;
use multihash::{Multihash, Sha2_256};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RadicleUri {
    hash: Multihash,
}

impl RadicleUri {
    pub fn new(hash: Multihash) -> Self {
        Self { hash }
    }
    pub fn hash(&self) -> &Multihash {
        &self.hash
    }

    pub fn from_str(s: &str) -> Result<Self, Error> {
        let bytes = bs58::decode(s.as_bytes())
            .with_alphabet(bs58::alphabet::BITCOIN)
            .into_vec()
            .map_err(|_| Error::InvalidBufferEncoding(s.to_owned()))?;
        let hash = Multihash::from_bytes(bytes).map_err(|_| Error::InvalidHash(s.to_owned()))?;
        Ok(Self { hash })
    }
}

lazy_static! {
    pub static ref EMPTY_HASH: Multihash = Sha2_256::digest(&[]);
    pub static ref EMPTY_URI: RadicleUri = RadicleUri::new(EMPTY_HASH.to_owned());
}

impl ToString for RadicleUri {
    fn to_string(&self) -> String {
        bs58::encode(&self.hash)
            .with_alphabet(bs58::alphabet::BITCOIN)
            .into_string()
    }
}

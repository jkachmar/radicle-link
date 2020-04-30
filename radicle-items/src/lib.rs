use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use std::{collections::BTreeMap, ops::Deref};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Item error ({0})")]
    ItemError(String),
}

pub fn msecs_from_epoch() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let from_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH");
    from_epoch.as_millis() as u64
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct BoolItem(bool);

impl Deref for BoolItem {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BoolItem {
    pub fn new(val: bool) -> Self {
        Self(val)
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub struct FloatItem(f64);

impl Deref for FloatItem {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FloatItem {
    pub fn new(val: f64) -> Self {
        Self(val)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct IntItem(i64);

impl Deref for IntItem {
    type Target = i64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntItem {
    pub fn new(val: i64) -> Self {
        Self(val)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct UIntItem(u64);

impl Deref for UIntItem {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UIntItem {
    pub fn new(val: u64) -> Self {
        Self(val)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct StringItem(String);

impl Deref for StringItem {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StringItem {
    pub fn new(val: String) -> Self {
        Self(val)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct BlobItem(Vec<u8>);

impl Deref for BlobItem {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BlobItem {
    pub fn new(val: Vec<u8>) -> Self {
        Self(val)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct UtcTimestampItem(u64);

impl Deref for UtcTimestampItem {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UtcTimestampItem {
    pub fn new(val: u64) -> Self {
        Self(val)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub struct TagItemId(String);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub struct UniqueItemId(Uuid);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub struct UniqueTimestampItemId(u64, Uuid);

#[derive(Clone, PartialEq, Serialize, Debug)]
pub struct ItemCollectionElement<ID>
where
    ID: Clone + PartialEq + Eq + Serialize + DeserializeOwned,
{
    id: ID,
    item: Item,
}

impl<ID> ItemCollectionElement<ID>
where
    ID: Clone + PartialEq + Eq + Serialize + DeserializeOwned,
{
    pub fn new(id: ID, item: Item) -> Self {
        Self { id, item }
    }

    pub fn id(&self) -> &ID {
        &self.id
    }
    pub fn item(&self) -> &Item {
        &self.item
    }
    pub fn item_mut(&mut self) -> &mut Item {
        &mut self.item
    }
}

impl<'de> Deserialize<'de> for ItemCollectionElement<TagItemId> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, SeqAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Item,
        };

        struct ItemCollectionElementVisitor;

        impl<'de> Visitor<'de> for ItemCollectionElementVisitor {
            type Value = ItemCollectionElement<TagItemId>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ItemCollectionElement")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<ItemCollectionElement<TagItemId>, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let id = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let item = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(ItemCollectionElement { id, item })
            }

            fn visit_map<V>(self, mut map: V) -> Result<ItemCollectionElement<TagItemId>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id = None;
                let mut item = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        },
                        Field::Item => {
                            if item.is_some() {
                                return Err(de::Error::duplicate_field("item"));
                            }
                            item = Some(map.next_value()?);
                        },
                    }
                }
                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let item = item.ok_or_else(|| de::Error::missing_field("item"))?;
                Ok(ItemCollectionElement { id, item })
            }
        }

        const FIELDS: &'static [&'static str] = &["id", "item"];
        deserializer.deserialize_struct(
            "ItemCollectionElement",
            FIELDS,
            ItemCollectionElementVisitor,
        )
    }
}

impl<'de> Deserialize<'de> for ItemCollectionElement<UniqueItemId> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, SeqAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Item,
        };

        struct ItemCollectionElementVisitor;

        impl<'de> Visitor<'de> for ItemCollectionElementVisitor {
            type Value = ItemCollectionElement<UniqueItemId>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ItemCollectionElement")
            }

            fn visit_seq<V>(
                self,
                mut seq: V,
            ) -> Result<ItemCollectionElement<UniqueItemId>, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let id = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let item = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(ItemCollectionElement { id, item })
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> Result<ItemCollectionElement<UniqueItemId>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id = None;
                let mut item = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        },
                        Field::Item => {
                            if item.is_some() {
                                return Err(de::Error::duplicate_field("item"));
                            }
                            item = Some(map.next_value()?);
                        },
                    }
                }
                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let item = item.ok_or_else(|| de::Error::missing_field("item"))?;
                Ok(ItemCollectionElement { id, item })
            }
        }

        const FIELDS: &'static [&'static str] = &["id", "item"];
        deserializer.deserialize_struct(
            "ItemCollectionElement",
            FIELDS,
            ItemCollectionElementVisitor,
        )
    }
}

impl<'de> Deserialize<'de> for ItemCollectionElement<UniqueTimestampItemId> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, SeqAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Item,
        };

        struct ItemCollectionElementVisitor;

        impl<'de> Visitor<'de> for ItemCollectionElementVisitor {
            type Value = ItemCollectionElement<UniqueTimestampItemId>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ItemCollectionElement")
            }

            fn visit_seq<V>(
                self,
                mut seq: V,
            ) -> Result<ItemCollectionElement<UniqueTimestampItemId>, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let id = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let item = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(ItemCollectionElement { id, item })
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> Result<ItemCollectionElement<UniqueTimestampItemId>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id = None;
                let mut item = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        },
                        Field::Item => {
                            if item.is_some() {
                                return Err(de::Error::duplicate_field("item"));
                            }
                            item = Some(map.next_value()?);
                        },
                    }
                }
                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let item = item.ok_or_else(|| de::Error::missing_field("item"))?;
                Ok(ItemCollectionElement { id, item })
            }
        }

        const FIELDS: &'static [&'static str] = &["id", "item"];
        deserializer.deserialize_struct(
            "ItemCollectionElement",
            FIELDS,
            ItemCollectionElementVisitor,
        )
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct StructItem {
    fields: BTreeMap<TagItemId, ItemCollectionElement<TagItemId>>,
}

impl StructItem {
    pub fn field(&self, id: &str) -> Option<&Item> {
        self.fields
            .get(&TagItemId(id.to_owned()))
            .map(|field| field.item())
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct BagItem {
    elements: BTreeMap<UniqueItemId, ItemCollectionElement<UniqueItemId>>,
}

impl BagItem {
    pub fn element(&self, id: &Uuid) -> Option<&Item> {
        self.elements
            .get(&UniqueItemId(id.to_owned()))
            .map(|element| element.item())
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SequenceItem {
    elements: Vec<ItemCollectionElement<UniqueItemId>>,
}

impl SequenceItem {
    pub fn element(&self, id: &Uuid) -> Option<&Item> {
        self.elements
            .iter()
            .find(|element| &element.id.0 == id)
            .map(|element| element.item())
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct LogItem {
    elements: BTreeMap<UniqueTimestampItemId, ItemCollectionElement<UniqueTimestampItemId>>,
}

impl LogItem {
    pub fn element(&self, timestamp: u64, id: &Uuid) -> Option<&Item> {
        self.elements
            .get(&UniqueTimestampItemId(timestamp, id.to_owned()))
            .map(|element| element.item())
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Item {
    Bool(BoolItem),
    Float(FloatItem),
    Int(IntItem),
    UInt(UIntItem),
    String(StringItem),
    Blob(BlobItem),
    UtcTimestamp(UtcTimestampItem),
    Struct(StructItem),
    Bag(BagItem),
    Sequence(SequenceItem),
    Log(LogItem),
}

pub struct OpReplace(Item);
pub struct OpInsert(ItemCollectionElement<UniqueItemId>);

pub struct OpRemove(UniqueItemId);

pub struct OpLogInsert(ItemCollectionElement<UniqueTimestampItemId>);

pub struct OpInsertBefore {
    anchor: Option<UniqueItemId>,
    item: Item,
}
pub struct OpInsertAfter {
    anchor: Option<UniqueItemId>,
    item: Item,
}

pub struct OpsOnField {
    id: TagItemId,
    ops: Vec<Operation>,
}

pub struct OpsOnElement {
    id: UniqueItemId,
    ops: Vec<Operation>,
}

pub struct OpsOnLogElement {
    id: UniqueTimestampItemId,
    ops: Vec<Operation>,
}

pub enum Operation {
    Replace(OpReplace),
    Insert(OpInsert),
    Remove(OpRemove),
    LogInsert(OpLogInsert),
    InsertBefore(OpInsertBefore),
    InsertAfter(OpInsertAfter),
    OnField(OpsOnField),
    OnElement(OpsOnElement),
    OnLogElement(OpsOnLogElement),
}

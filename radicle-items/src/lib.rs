use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use std::{collections::BTreeMap, ops::Deref};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Item error ({0})")]
    ItemError(String),
    #[error("Unsupported operation ({0})")]
    UnsupportedOperation(&'static str),
    #[error("Unsupported operand ({0})")]
    UnsupportedOperand(&'static str),
}

pub type ItemResult = Result<(), Error>;

pub fn msecs_from_epoch() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let from_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH");
    from_epoch.as_millis() as u64
}

pub trait ItemExt {
    fn kind(&self) -> &'static str;
    fn apply(&mut self, op: &Operation) -> ItemResult;
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct BoolItem(bool);

impl Deref for BoolItem {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ItemExt for BoolItem {
    fn kind(&self) -> &'static str {
        "bool"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::Replace(op) => {
                let operand = &op.0;
                match operand {
                    Item::Bool(val) => self.replace(val),
                    _ => Err(Error::UnsupportedOperand(operand.kind())),
                }
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl BoolItem {
    pub fn new(val: bool) -> Self {
        Self(val)
    }

    pub fn replace(&mut self, val: &Self) -> ItemResult {
        *self = *val;
        Ok(())
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

impl ItemExt for FloatItem {
    fn kind(&self) -> &'static str {
        "float"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::Replace(op) => {
                let operand = &op.0;
                match operand {
                    Item::Float(val) => self.replace(val),
                    _ => Err(Error::UnsupportedOperand(operand.kind())),
                }
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl FloatItem {
    pub fn new(val: f64) -> Self {
        Self(val)
    }

    pub fn replace(&mut self, val: &Self) -> ItemResult {
        *self = *val;
        Ok(())
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

impl ItemExt for IntItem {
    fn kind(&self) -> &'static str {
        "int"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::Replace(op) => {
                let operand = &op.0;
                match operand {
                    Item::Int(val) => self.replace(val),
                    _ => Err(Error::UnsupportedOperand(operand.kind())),
                }
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl IntItem {
    pub fn new(val: i64) -> Self {
        Self(val)
    }

    pub fn replace(&mut self, val: &Self) -> ItemResult {
        *self = *val;
        Ok(())
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

impl ItemExt for UIntItem {
    fn kind(&self) -> &'static str {
        "uint"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::Replace(op) => {
                let operand = &op.0;
                match operand {
                    Item::UInt(val) => self.replace(val),
                    _ => Err(Error::UnsupportedOperand(operand.kind())),
                }
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl UIntItem {
    pub fn new(val: u64) -> Self {
        Self(val)
    }

    pub fn replace(&mut self, val: &Self) -> ItemResult {
        *self = *val;
        Ok(())
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

impl ItemExt for StringItem {
    fn kind(&self) -> &'static str {
        "string"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::Replace(op) => {
                let operand = &op.0;
                match operand {
                    Item::String(val) => self.replace(val),
                    _ => Err(Error::UnsupportedOperand(operand.kind())),
                }
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl StringItem {
    pub fn new(val: String) -> Self {
        Self(val)
    }

    pub fn replace(&mut self, val: &Self) -> ItemResult {
        self.0 = val.0.clone();
        Ok(())
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

impl ItemExt for BlobItem {
    fn kind(&self) -> &'static str {
        "blob"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::Replace(op) => {
                let operand = &op.0;
                match operand {
                    Item::Blob(val) => self.replace(val),
                    _ => Err(Error::UnsupportedOperand(operand.kind())),
                }
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl BlobItem {
    pub fn new(val: Vec<u8>) -> Self {
        Self(val)
    }

    pub fn replace(&mut self, val: &Self) -> ItemResult {
        self.0 = val.0.clone();
        Ok(())
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

impl ItemExt for UtcTimestampItem {
    fn kind(&self) -> &'static str {
        "timestamp"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::Replace(op) => {
                let operand = &op.0;
                match operand {
                    Item::UtcTimestamp(val) => self.replace(val),
                    _ => Err(Error::UnsupportedOperand(operand.kind())),
                }
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl UtcTimestampItem {
    pub fn new(val: u64) -> Self {
        Self(val)
    }

    pub fn replace(&mut self, val: &Self) -> ItemResult {
        *self = *val;
        Ok(())
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
    pub(crate) fn item_mut(&mut self) -> &mut Item {
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

impl ItemExt for StructItem {
    fn kind(&self) -> &'static str {
        "struct"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::OnField(op) => {
                for o in op.ops.iter() {
                    self.apply_to_field(&op.id.0, o)?
                }
                Ok(())
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl StructItem {
    pub fn field(&self, id: &str) -> Option<&Item> {
        self.fields
            .get(&TagItemId(id.to_owned()))
            .map(|field| field.item())
    }

    pub fn apply_to_field(&mut self, id: &str, op: &Operation) -> ItemResult {
        match self.fields.get_mut(&TagItemId(id.to_owned())) {
            Some(field) => field.item_mut().apply(op),
            None => Err(Error::ItemError(String::from("Missing field"))),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct BagItem {
    elements: BTreeMap<UniqueItemId, ItemCollectionElement<UniqueItemId>>,
}

impl ItemExt for BagItem {
    fn kind(&self) -> &'static str {
        "bag"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::Insert(op) => {
                let element = &op.0;
                self.insert(element.id.0, element.item.clone())
            },
            Operation::Remove(op) => self.remove((op.0).0),
            Operation::OnElement(op) => {
                for o in op.ops.iter() {
                    self.apply_to_element(&op.id.0, o)?
                }
                Ok(())
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl BagItem {
    pub fn element(&self, id: &Uuid) -> Option<&Item> {
        self.elements
            .get(&UniqueItemId(id.to_owned()))
            .map(|element| element.item())
    }

    pub fn apply_to_element(&mut self, id: &Uuid, op: &Operation) -> ItemResult {
        match self.elements.get_mut(&UniqueItemId(id.to_owned())) {
            Some(element) => element.item_mut().apply(op),
            None => Err(Error::ItemError(String::from("Missing element"))),
        }
    }

    pub fn insert(&mut self, id: Uuid, item: Item) -> ItemResult {
        let element = ItemCollectionElement::<UniqueItemId> {
            id: UniqueItemId(id),
            item,
        };
        self.elements.insert(UniqueItemId(id), element);
        Ok(())
    }

    pub fn remove(&mut self, id: Uuid) -> ItemResult {
        self.elements.remove(&UniqueItemId(id));
        Ok(())
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SequenceItem {
    elements: Vec<ItemCollectionElement<UniqueItemId>>,
}

impl ItemExt for SequenceItem {
    fn kind(&self) -> &'static str {
        "sequence"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::InsertAfter(op) => {
                self.insert_after(&op.anchor, op.item.id.0, op.item.item.clone())
            },
            Operation::InsertBefore(op) => {
                self.insert_before(&op.anchor, op.item.id.0, op.item.item.clone())
            },
            Operation::Remove(op) => self.remove((op.0).0),
            Operation::OnElement(op) => {
                for o in op.ops.iter() {
                    self.apply_to_element(&op.id.0, o)?
                }
                Ok(())
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl SequenceItem {
    pub fn element(&self, id: &Uuid) -> Option<&Item> {
        self.elements
            .iter()
            .find(|element| &element.id.0 == id)
            .map(|element| element.item())
    }

    fn element_mut(&mut self, id: &Uuid) -> Option<&mut Item> {
        self.elements
            .iter_mut()
            .find(|element| &element.id.0 == id)
            .map(|element| element.item_mut())
    }

    pub fn apply_to_element(&mut self, id: &Uuid, op: &Operation) -> ItemResult {
        match self.element_mut(id) {
            Some(element) => element.apply(op),
            None => Err(Error::ItemError(String::from("Missing element"))),
        }
    }

    pub fn insert_before(
        &mut self,
        anchor: &Option<UniqueItemId>,
        id: Uuid,
        item: Item,
    ) -> ItemResult {
        let element = ItemCollectionElement::<UniqueItemId> {
            id: UniqueItemId(id),
            item,
        };
        let index = match anchor {
            Some(_) => unimplemented!(),
            None => self.elements.len(),
        };
        self.elements.insert(index, element);
        Ok(())
    }

    pub fn insert_after(
        &mut self,
        anchor: &Option<UniqueItemId>,
        id: Uuid,
        item: Item,
    ) -> ItemResult {
        let element = ItemCollectionElement::<UniqueItemId> {
            id: UniqueItemId(id),
            item,
        };
        let index = match anchor {
            Some(_) => unimplemented!(),
            None => 0,
        };
        self.elements.insert(index, element);
        Ok(())
    }

    pub fn remove(&mut self, _id: Uuid) -> ItemResult {
        unimplemented!()
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct LogItem {
    elements: BTreeMap<UniqueTimestampItemId, ItemCollectionElement<UniqueTimestampItemId>>,
}

impl ItemExt for LogItem {
    fn kind(&self) -> &'static str {
        "log"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match op {
            Operation::LogInsert(op) => {
                let element = &op.0;
                self.insert(element.id.0, element.id.1, element.item.clone())
            },
            Operation::OnLogElement(op) => {
                for o in op.ops.iter() {
                    self.apply_to_element(op.id.0, &op.id.1, o)?
                }
                Ok(())
            },
            _ => Err(Error::UnsupportedOperation(op.kind())),
        }
    }
}

impl LogItem {
    pub fn element(&self, timestamp: u64, id: &Uuid) -> Option<&Item> {
        self.elements
            .get(&UniqueTimestampItemId(timestamp, id.to_owned()))
            .map(|element| element.item())
    }

    pub fn apply_to_element(&mut self, timestamp: u64, id: &Uuid, op: &Operation) -> ItemResult {
        match self
            .elements
            .get_mut(&UniqueTimestampItemId(timestamp, id.to_owned()))
        {
            Some(element) => element.item_mut().apply(op),
            None => Err(Error::ItemError(String::from("Missing element"))),
        }
    }

    pub fn insert(&mut self, timestamp: u64, id: Uuid, item: Item) -> ItemResult {
        let element = ItemCollectionElement::<UniqueTimestampItemId> {
            id: UniqueTimestampItemId(timestamp, id),
            item,
        };
        self.elements
            .insert(UniqueTimestampItemId(timestamp, id), element);
        Ok(())
    }

    // FIXME: do we need this?
    pub fn remove(&mut self, timestamp: u64, id: Uuid) -> ItemResult {
        self.elements.remove(&UniqueTimestampItemId(timestamp, id));
        Ok(())
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

impl ItemExt for Item {
    fn kind(&self) -> &'static str {
        "item"
    }

    fn apply(&mut self, op: &Operation) -> ItemResult {
        match self {
            Item::Bool(item) => item.apply(op),
            Item::Float(item) => item.apply(op),
            Item::Int(item) => item.apply(op),
            Item::UInt(item) => item.apply(op),
            Item::String(item) => item.apply(op),
            Item::Blob(item) => item.apply(op),
            Item::UtcTimestamp(item) => item.apply(op),
            Item::Struct(item) => item.apply(op),
            Item::Bag(item) => item.apply(op),
            Item::Sequence(item) => item.apply(op),
            Item::Log(item) => item.apply(op),
        }
    }
}

pub struct OpReplace(Item);
pub struct OpInsert(ItemCollectionElement<UniqueItemId>);

pub struct OpRemove(UniqueItemId);

pub struct OpLogInsert(ItemCollectionElement<UniqueTimestampItemId>);

pub struct OpInsertBefore {
    anchor: Option<UniqueItemId>,
    item: ItemCollectionElement<UniqueItemId>,
}
pub struct OpInsertAfter {
    anchor: Option<UniqueItemId>,
    item: ItemCollectionElement<UniqueItemId>,
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

pub trait OperationExt {
    fn kind(&self) -> &'static str;
}

impl OperationExt for Operation {
    fn kind(&self) -> &'static str {
        "operation"
    }
}

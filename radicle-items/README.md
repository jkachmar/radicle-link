# Item (this is the "state")

For simplicity, a `serde_json::value::Value`

- ID: a scoped key for item components
  - Tag: String
  - UniqueID: Blob (its String representation is its base64 value)
  - UniqueTimestamp: Timestamp + UniqueID (the strings are concatenated with a separator)
- Primitive
  - Null
  - Bool
  - Number
  - String
  - Blob: base64 String
  - UtcTimestamp as epoch number
- Enum: `struct { tag: Tag, val: Item }`
- CollectionElement: `struct { key: ID, val: Item }`
- Collection
  - Struct: Object (field ids are `Tag`)
  - Set: Object (`Set<CollectionElement<UniqueID>>`)
  - Sequence
    - Timestamp: `Vec<CollectionElement<UniqueTimestamp>>`
    - Natural `Vec<CollectionElement<UniqueID>>`

# Patch (this is the "operation")

A value that describes how to turn an Item into another Item.
`enum { Primitive, Enum, Collection }`

## Primitive

Replace with new value
`struct (PrimitiveItem)`

## Enum

- If same branch: patch the value (it carries the Tag for safety)
- Otherwise: replace with new value

`enum { Same(Tag, Patch), Other(EnumItem) }`

## Collection

### Struct

Set of field patches

`Set<struct { tag: Tag, patch: Patch }>`

### Set

Three operations (each operates on a single element):

- Add `struct { id: UniqueID, val: Item }`
- Remove `UniqueId`
- Modify `struct { id: UniqueID, patch: Patch }`

`enum { Add, Remove, Modify }`

### Sequence

Four operations (each operates on a single element):

- AddBefore `struct { before: Option<UniqueID>, id: UniqueID, val: Item }`
- AddAfter `struct { after: Option<UniqueID>, id: UniqueID, val: Item }`
- Remove `UniqueId`
- Modify `struct { id: UniqueID, patch: Patch }`

`enum { AddBefore. AddAfter, Remove, Modify }`

# Snapshot format

`flart` never talks to Figma itself. The collector script (`collector/collect.js`, printed by `flart collector`) runs inside a Figma file through the Plugin API and returns one **part**. `flart snapshot assemble` checks the parts and merges them into one **snapshot**, and every other command reads that snapshot.

Field names follow the Plugin API (`valuesByMode`, `resolvedType`, `fontName`, `lineHeight`, `boundVariables`, …), so a value in a snapshot means what the same property means in the [Plugin API docs](https://www.figma.com/plugin-docs/api/api-reference/). Both file types carry `schemaVersion: 1`, and `flart` refuses any other version.

## Parts

A part is the object the collector returns:

```json
{
  "flart": "snapshot-part",
  "schemaVersion": 1,
  "part": "variables:VariableCollectionId:1:12",
  "bytes": 40699,
  "checksum": "fnv1a32:5f31e27a",
  "data": { "collectionId": "VariableCollectionId:1:12", "variables": [] }
}
```

`bytes` is the UTF-8 length of `JSON.stringify(data)` and `checksum` is the 32-bit FNV-1a hash of those bytes, both computed inside Figma. `bytes` is informational; `assemble` checks only the checksum. It serializes `data` the way `JSON.stringify` does (keys in the order they appear, numbers in JavaScript's shortest round-trip form) and compares. Whitespace and line breaks added while saving the part do not matter. A changed digit, a rounded number, or a dropped field does, and fails with exit code 4.

A snapshot needs these parts, each exactly once:

| `part` | `data` |
| --- | --- |
| `collections` | `collections`: the file's local variable collections. |
| `variables:<collection id>` | `collectionId` and `variables`, for one local collection. One part per collection listed in `collections`. |
| `styles` | `textStyles`, `paintStyles`, `effectStyles`: the file's local styles. |
| `remote` | `collections`, `variables`, `missing`: library variables that local variables or styles alias, followed transitively, with their collections. `missing` lists alias ids the file could not load. |

Variables are split by collection because `use_figma` returns one value per call, and a large collection with several modes is tens of kilobytes on its own.

## Snapshot

`assemble` writes:

```json
{
  "flart": "snapshot",
  "schemaVersion": 1,
  "collections": [],
  "variables": [],
  "textStyles": [],
  "paintStyles": [],
  "effectStyles": [],
  "missing": []
}
```

`collections` holds the local collections first, in Figma's order, then the library collections from `remote`. `variables` follows the same order. A library collection has `remote: true` and an empty `variableIds`, because only the variables that aliases reach are collected.

Besides the Plugin API properties, collections and variables keep `id`, `key`, `remote`, and, for collections, `defaultModeId` and `modes` (`modeId`, `name`). Code generation needs `defaultModeId`: an alias into a collection with several modes resolves in that collection's default mode, as Figma resolves it for a node with no explicit mode.

## Size

An agent reads every part from `use_figma` and writes it to a file, so the cost of a run grows with the parts. A file with 101 variables in two collections (five modes and two), 52 library variables, and 20 styles came to 82 KB of parts, most of it in the five-mode collection. Every variable repeats its `key` and `variableCollectionId`, and every mode repeats the full alias id. Dropping the first two and interning alias ids would roughly halve that, at the price of `schemaVersion: 2`. That change is worth making once a part fails to come back whole from `use_figma` (the 41 KB part above returned intact), or once regeneration runs often enough for the cost to matter.

## What `assemble` and `check` reject

| Exit | Problem |
| --- | --- |
| 3 | A part is missing, appears twice, names a collection that is not local, or has the wrong marker or version. An alias chain loops back on itself. |
| 4 | A part's checksum does not match its `data`. |
| 5 | A variable or style aliases a variable that is not in the snapshot, or `missing` is not empty. |

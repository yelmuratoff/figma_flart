const PART = "__FLART_PART__";
const SCHEMA_VERSION = 1;

const pick = (source, keys) => {
  const out = {};
  for (const key of keys) {
    if (source[key] !== undefined) out[key] = source[key];
  }
  return out;
};

const clone = (value) => JSON.parse(JSON.stringify(value));

const utf8 = (text) => {
  const bytes = [];
  for (let i = 0; i < text.length; i++) {
    let code = text.charCodeAt(i);
    if (code >= 0xd800 && code <= 0xdbff && i + 1 < text.length) {
      const next = text.charCodeAt(i + 1);
      if (next >= 0xdc00 && next <= 0xdfff) {
        code = 0x10000 + ((code - 0xd800) << 10) + (next - 0xdc00);
        i++;
      }
    }
    if (code < 0x80) bytes.push(code);
    else if (code < 0x800) bytes.push(0xc0 | (code >> 6), 0x80 | (code & 63));
    else if (code < 0x10000) bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 63), 0x80 | (code & 63));
    else bytes.push(0xf0 | (code >> 18), 0x80 | ((code >> 12) & 63), 0x80 | ((code >> 6) & 63), 0x80 | (code & 63));
  }
  return bytes;
};

const fnv1a32 = (text) => {
  let hash = 0x811c9dc5;
  for (const byte of utf8(text)) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
};

const collectionData = (collection) => ({
  id: collection.id,
  name: collection.name,
  key: collection.key,
  remote: collection.remote,
  defaultModeId: collection.defaultModeId,
  modes: collection.modes.map((mode) => ({ modeId: mode.modeId, name: mode.name })),
  variableIds: collection.variableIds.slice(),
});

const variableData = (variable) => ({
  id: variable.id,
  name: variable.name,
  key: variable.key,
  remote: variable.remote,
  variableCollectionId: variable.variableCollectionId,
  resolvedType: variable.resolvedType,
  valuesByMode: clone(variable.valuesByMode),
});

const aliasIds = (value, into) => {
  if (Array.isArray(value)) {
    for (const item of value) aliasIds(item, into);
  } else if (value && typeof value === "object") {
    if (value.type === "VARIABLE_ALIAS" && typeof value.id === "string") into.push(value.id);
    for (const key of Object.keys(value)) aliasIds(value[key], into);
  }
};

const styles = async () => {
  const text = await figma.getLocalTextStylesAsync();
  const paint = await figma.getLocalPaintStylesAsync();
  const effect = await figma.getLocalEffectStylesAsync();
  return {
    textStyles: text.map((style) => ({
      ...pick(style, ["id", "name", "fontSize", "textDecoration"]),
      fontName: clone(style.fontName),
      lineHeight: clone(style.lineHeight),
      letterSpacing: clone(style.letterSpacing),
      boundVariables: clone(style.boundVariables || {}),
    })),
    paintStyles: paint.map((style) => ({ id: style.id, name: style.name, paints: clone(style.paints) })),
    effectStyles: effect.map((style) => ({ id: style.id, name: style.name, effects: clone(style.effects) })),
  };
};

const collections = async () => {
  const local = await figma.variables.getLocalVariableCollectionsAsync();
  return { collections: local.map(collectionData) };
};

const variablesOf = async (collectionId) => {
  const collection = await figma.variables.getVariableCollectionByIdAsync(collectionId);
  if (!collection) throw new Error(`collection ${collectionId} not found`);
  const variables = [];
  for (const id of collection.variableIds) {
    const variable = await figma.variables.getVariableByIdAsync(id);
    if (variable) variables.push(variableData(variable));
  }
  return { collectionId, variables };
};

const remote = async () => {
  const localCollections = await figma.variables.getLocalVariableCollectionsAsync();
  const known = new Set();
  const pending = [];
  for (const collection of localCollections) {
    for (const id of collection.variableIds) {
      known.add(id);
      const variable = await figma.variables.getVariableByIdAsync(id);
      if (variable) aliasIds(variable.valuesByMode, pending);
    }
  }
  aliasIds(await styles(), pending);

  const variables = [];
  const collectionsById = new Map();
  const missing = [];
  while (pending.length > 0) {
    const id = pending.pop();
    if (known.has(id)) continue;
    known.add(id);
    const variable = await figma.variables.getVariableByIdAsync(id);
    if (!variable) {
      missing.push(id);
      continue;
    }
    variables.push(variableData(variable));
    aliasIds(variable.valuesByMode, pending);
    if (!collectionsById.has(variable.variableCollectionId)) {
      const collection = await figma.variables.getVariableCollectionByIdAsync(variable.variableCollectionId);
      if (collection) {
        const data = collectionData(collection);
        data.variableIds = [];
        collectionsById.set(collection.id, data);
      }
    }
  }
  variables.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return { collections: Array.from(collectionsById.values()), variables, missing: missing.sort() };
};

const collect = async () => {
  if (PART === "styles") return styles();
  if (PART === "collections") return collections();
  if (PART === "remote") return remote();
  if (PART.startsWith("variables:")) return variablesOf(PART.slice("variables:".length));
  throw new Error(`unknown part ${PART}`);
};

const data = await collect();
const canonical = JSON.stringify(data);
return {
  flart: "snapshot-part",
  schemaVersion: SCHEMA_VERSION,
  part: PART,
  bytes: utf8(canonical).length,
  checksum: `fnv1a32:${fnv1a32(canonical)}`,
  data,
};

// Signs hand-written snapshot parts the way collect.js does.
const fs = require("fs");
const path = require("path");

const dir = process.argv[2];
for (const name of fs.readdirSync(dir).filter((n) => n.endsWith(".json"))) {
  const file = path.join(dir, name);
  const part = JSON.parse(fs.readFileSync(file, "utf8"));
  const bytes = Buffer.from(JSON.stringify(part.data), "utf8");
  let hash = 0x811c9dc5;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193);
  }
  part.bytes = bytes.length;
  part.checksum = "fnv1a32:" + (hash >>> 0).toString(16).padStart(8, "0");
  fs.writeFileSync(file, JSON.stringify(part) + "\n");
  console.log(name, part.checksum, part.bytes);
}

# Oracle

The golden files in `tests/golden/` come from the Flart 1.1.1 generators in `../src/generators`, run unchanged in Node. `figma-shim.ts` installs a fake `figma` global that answers the six Plugin API calls those generators make from an assembled snapshot. `run.ts` calls every generator with every flag combination and writes one `.dart` file per combination. `tests/golden.rs` then requires `flart generate` to match each file byte for byte, apart from the divergences it lists.

The golden files drop the `// Last updated: …` line, which `flart` does not write.

## Regenerating goldens

Run these from the repository root, one at a time. `<case>` is `sample` (committed) or `ah_ds` (local only, see below).

```sh
cli/target/debug/flart snapshot assemble cli/tests/fixtures/<case>/parts/*.json -o cli/tests/fixtures/<case>/snapshot.json
node_modules/.bin/esbuild cli/oracle/run.ts --bundle --platform=node --outfile=$TMPDIR/flart-oracle.js
node $TMPDIR/flart-oracle.js cli/tests/fixtures/<case>/snapshot.json cli/tests/golden/<case>
```

File names encode the flags: `theme_ext_on|off`, `font_name_on|off`, and `modes_none|first|all`. The `first` and `all` variants exist only for a kind whose mode source has at least two modes.

Regenerate only when the 1.1.1 generators or a fixture change. A golden that changes for any other reason points at the shim.

## Hand-written parts

`sign-parts.js <dir>` fills in `bytes` and `checksum` for every part in a directory, the same way the collector does. Use it after editing a fixture part by hand:

```sh
node cli/oracle/sign-parts.js cli/tests/fixtures/sample/parts
```

## Fixtures

`sample` is synthetic and committed. It has the same shape as a real design system file: two local collections (three modes and two), a single-mode library collection reached through aliases, string and boolean variables, text styles bound to a multi-mode collection, a gradient paint style with bound stops, and shadow and blur effects.

`ah_ds` is a real client file. It stays on the machine that captured it: `tests/fixtures/ah_ds/` and `tests/golden/ah_ds/` are in `.gitignore`, and the tests that read them print a skip line when the directory is absent.

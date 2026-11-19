<p align="center"><img src="./assets/icon.png" align="center" alt="Flart logo" width="128" height="128"></p>
  
<h1 align="center">Flart: Flutter / Dart generator</h1>

<div align="center">
<a href="https://www.figma.com/community/plugin/1419440139622021656/Flart" align="center"><img src="assets/install_button.png" align="center" alt="Install Plugin"></a>
</div>

<br />

## Problem

The process of translating text styles and colors from Figma to Flutter can lead to several issues, including:

- The need for manual conversion of Figma text styles into Flutter code
- Increased risk of inconsistencies between the design files and the implemented application
- Manual conversion often results in code duplication
- Errors such as incorrect values, typos, or missing styles can occur, which can be costly and time-consuming to fix
- Every time a designer updates a text style in Figma, developers must manually update the corresponding code, which is both inefficient and prone to errors

## Solution

Automating the conversion process eliminates these issues, making the design-to-code workflow more efficient, accurate, and consistent.

## Command-line generator

`cli/` holds `flart`, a Rust command-line version of the plugin's generators for files where you cannot run plugins, such as a Dev seat or view-only access. A read-only script collects the file's variables and styles through the Figma MCP server's `use_figma` tool, and `flart` turns that snapshot into the same Dart the plugin writes.

```bash
cargo install --path cli --locked

flart collector --part collections        # script to run through use_figma
flart snapshot assemble parts/*.json -o snapshot.json
flart generate snapshot.json --out lib/theme/generated --theme-extensions --font-name
flart export snapshot.json --out design-tokens   # DTCG 2025.10 for any other stack
```

`flart export` writes the same snapshot in the [Design Tokens Community Group format 2025.10](https://www.designtokens.org/tr/2025.10/format/): a token file per collection, one per mode where a collection has several, and a resolver document that picks a mode per collection. A token tool that reads DTCG, such as Style Dictionary or Terrazzo, turns it into the target platform's code. [`cli/docs/dtcg-export.md`](cli/docs/dtcg-export.md) lists how each Figma value maps and what DTCG cannot express.

`flart --help` lists every option, and `--json` switches any command to machine-readable output. The output matches plugin 1.1.1 except that it has no `// Last updated` line, aliases into multi-mode collections resolve in the collection's default mode, and gradient stops keep their alpha. `flart` does not run `dart format`; run it yourself if your project formats generated files. [`cli/docs/snapshot-schema.md`](cli/docs/snapshot-schema.md) describes the snapshot, and [`cli/oracle/README.md`](cli/oracle/README.md) explains how the tests compare against the plugin.

## Getting Started

1.  **Install dependencies:**

    ```bash
    npm install
    ```

2.  **Build the plugin:**

    ```bash
    npm run build
    ```

3.  **Development mode (watch):**
    ```bash
    npm run watch
    ```

## How to test in Figma

1.  Open the **Figma Desktop app**.
2.  Go to **Plugins** -> **Development** -> **Import plugin from manifest...**.
3.  Select the `manifest.json` file in this project's root directory.
4.  To open the plugin: **Right-click** on canvas -> **Plugins** -> **Development** -> **Flart**.
5.  To debug: **Right-click** on the plugin window -> **Inspect** (this opens the developer console for the UI) or use the Figma Desktop console (**Plugins** -> **Development** -> **Open Console**).

## Publishing new version

1.  **Build the project:**
    ```bash
    npm run build
    ```
2.  **Bump version (optional):** Update the version in `package.json`.
3.  In Figma Desktop: Go to **Plugins** -> **Manage plugins...**.
4.  Find **Flart** in the **Development** section.
5.  Click the **...** (three dots) and select **Publish**.
6.  If it's an update, select **Publish new version**.
7.  Fill in the **Release notes** to describe your changes.
8.  Click **Publish** (it will be available immediately if already approved, or sent for re-review if permissions changed).

### TODO

- [x] Tokens/Variables
- [x] Text decoration
- [x] Letter spacing
- [x] Line height.

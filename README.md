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

### TODO

- [x] Tokens/Variables
- [x] Text decoration
- [x] Letter spacing
- [x] Line height.

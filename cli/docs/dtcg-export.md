# DTCG export

`flart export <snapshot> --out <dir>` writes the snapshot in the Design Tokens Community Group format 2025.10 ([Format](https://www.designtokens.org/tr/2025.10/format/), [Color](https://www.designtokens.org/tr/2025.10/color/), and [Resolver](https://www.designtokens.org/tr/2025.10/resolver/) modules). Use it when the project is not Flutter, or when a token tool such as Style Dictionary or Terrazzo builds the platform code.

## Files

```text
<dir>/
  tokens.resolver.json              modes: one modifier per multi-mode collection
  tokens/<collection>.tokens.json   a collection with one mode (library collections too)
  tokens/<collection>/<mode>.tokens.json
  tokens/styles.tokens.json         paint, effect, and text styles
```

Every token sits under its collection's name, followed by the Figma name split at `/`: the variable `text/strong` in the collection `Theme` becomes `Theme.text.strong`. Styles sit under `text-styles`, `paint-styles`, and `effect-styles`. Names keep their Figma spelling; the token tool applies its own naming. DTCG forbids `.`, `{`, and `}` in names, so they become `_`, and a leading `$` becomes `_`. When a Figma name is both a token and a group (`border` and `border/width`), the token moves to `border.$root`. Two names that end up the same fail with exit code 7.

The resolver lists the single-mode files as sets and each multi-mode collection as a modifier whose contexts are its modes, with Figma's default mode as `default`. A tool resolves one combination at a time, for example `{ "Theme": "dark", "Size": "Mobile" }`.

## Values

| Figma | DTCG |
| --- | --- |
| COLOR variable | `color`: `srgb` components and alpha as Figma stores them, plus `hex` |
| FLOAT variable | `dimension` in `px` |
| Alias | a reference such as `{Palette.gray.900}`, resolved by the tool for the selected modes |
| Solid paint style | `color`; a reference when the paint is bound to a variable and has no opacity of its own |
| Linear gradient paint style | `gradient` stops with color and position |
| Drop and inner shadows | `shadow`, an array when the style has several; `inset` for inner shadows; bound fields become references |
| Text style | `typography` with literal values; see below |

A text style bound to a variable in a multi-mode collection is written into that collection's mode files with the values of each mode. DTCG typography needs `lineHeight` as a ratio of the font size, which no reference to a pixel variable can express, so the values are resolved per mode instead. Figma line heights in pixels become `px / fontSize`, percentages become `value / 100`, and letter spacing in percent becomes pixels of the font size. These computed values are rounded to four decimals. The font weight comes from the style name (`Bold` is 700, `Semi Bold` is 600), as in the Dart output.

## What is dropped

DTCG 2025.10 has no type for some Figma values. `flart` leaves them out and lists each one under `dropped` in the output, with the reason:

- STRING and BOOLEAN variables.
- Layer and background blur effects.
- Paint styles with more than one paint, and paint types other than solid and linear gradient.
- Text styles with an AUTO line height, which depends on the font's metrics.

A gradient's angle and a text style's decoration are not part of the DTCG values either, and are not reported. A FLOAT variable that holds something other than a length, such as an opacity, still becomes a `px` dimension, because the snapshot does not carry Figma's variable scopes.

## Checking the output

The output validates against the published JSON Schemas at `https://www.designtokens.org/schemas/2025.10/format.json` and `.../resolver.json`. The schemas do not check that references point at existing tokens; `tests/export.rs` does that for the sample.

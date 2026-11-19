import { mkdirSync, readFileSync, writeFileSync } from "fs";
import { join } from "path";

import { installFigma } from "./figma-shim";
import { collectColorModes, generateColors } from "../../src/generators/generateColors";
import {
  collectEffectModes,
  generateEffectStyles,
} from "../../src/generators/generateEffectStyles";
import {
  collectTextStyleModes,
  generateTextStyles,
} from "../../src/generators/generateTextStyles";
import {
  collectVariableModes,
  generateVariables,
} from "../../src/generators/generateVariables";

type ModeInfo = { modeId: string; name: string };
type Selection = { label: string; modeIds?: string[] };

const [snapshotPath, outDir] = process.argv.slice(2);
if (!snapshotPath || !outDir) {
  console.error("usage: node flart-oracle.js <snapshot.json> <golden dir>");
  process.exit(2);
}

console.log = () => {};
installFigma(JSON.parse(readFileSync(snapshotPath, "utf8")));

const stripTimestamp = (code: string) =>
  code
    .split("\n")
    .filter((line) => !line.startsWith("// Last updated: "))
    .join("\n");

const selections = (modes: ModeInfo[] | null): Selection[] => {
  const result: Selection[] = [{ label: "modes_none" }];
  if (modes && modes.length > 1) {
    result.push({ label: "modes_first", modeIds: [modes[0].modeId] });
    result.push({ label: "modes_all", modeIds: modes.map((m) => m.modeId) });
  }
  return result;
};

const write = (kind: string, flags: string[], code: string) => {
  mkdirSync(join(outDir, kind), { recursive: true });
  writeFileSync(join(outDir, kind, `${flags.join("-")}.dart`), stripTimestamp(code));
};

const flag = (name: string, on: boolean) => `${name}_${on ? "on" : "off"}`;

const main = async () => {
  for (const mode of selections(await collectColorModes())) {
    write("colors", [mode.label], await generateColors(mode.modeIds));
  }
  for (const mode of selections(await collectEffectModes())) {
    write("effects", [mode.label], await generateEffectStyles(mode.modeIds));
  }
  for (const mode of selections(await collectTextStyleModes())) {
    for (const themeExt of [false, true]) {
      for (const fontName of [false, true]) {
        const code = await generateTextStyles(themeExt, fontName, mode.modeIds);
        write("text_styles", [flag("theme_ext", themeExt), flag("font_name", fontName), mode.label], code);
      }
    }
  }
  for (const mode of selections(await collectVariableModes())) {
    for (const themeExt of [false, true]) {
      const code = await generateVariables(themeExt, mode.modeIds);
      write("variables", [flag("theme_ext", themeExt), mode.label], code);
    }
  }
};

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

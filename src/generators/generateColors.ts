import {
  initialize,
  generateColorStyleDartCode,
  formatColorName,
  formatClassName,
  toHex,
} from "../utils";

interface ModeInfo {
  modeId: string;
  name: string;
}

/**
 * Collects modes from paint styles that have bound variables on their paints.
 */
export async function collectColorModes(): Promise<ModeInfo[] | null> {
  const styles = await figma.getLocalPaintStylesAsync();

  for (const style of styles) {
    const paint = style.paints[0] as any;
    if (!paint?.boundVariables?.color) continue;

    const variable = await figma.variables.getVariableByIdAsync(
      paint.boundVariables.color.id
    );
    if (!variable) continue;

    const collection = await figma.variables.getVariableCollectionByIdAsync(
      variable.variableCollectionId
    );
    if (!collection || collection.modes.length <= 1) continue;

    return collection.modes.map((m) => ({
      modeId: m.modeId,
      name: m.name,
    }));
  }
  return null;
}

async function resolveColorForMode(
  paint: any,
  modeId: string
): Promise<{ r: number; g: number; b: number; a: number } | null> {
  if (!paint?.boundVariables?.color) return null;

  const variable = await figma.variables.getVariableByIdAsync(
    paint.boundVariables.color.id
  );
  if (!variable) return null;

  const val = variable.valuesByMode[modeId];
  if (!val || typeof val !== "object" || !("r" in val)) return null;

  return {
    r: (val as any).r,
    g: (val as any).g,
    b: (val as any).b,
    a: "a" in val ? (val as any).a : 1,
  };
}

async function generateColorsForMode(
  styles: PaintStyle[],
  modeId: string,
  modeName: string,
  useModeSuffix: boolean
): Promise<string> {
  const suffix = useModeSuffix ? formatClassName(modeName) : "";
  let code = `abstract class App${suffix}Colors {\n`;

  for (let index = 0; index < styles.length; index++) {
    const style = styles[index];
    const paint = style.paints[0];

    if (paint.type === "SOLID") {
      let r = paint.color.r;
      let g = paint.color.g;
      let b = paint.color.b;
      let opacity = paint.opacity || 1;

      const modeColor = await resolveColorForMode(paint as any, modeId);
      if (modeColor) {
        r = modeColor.r;
        g = modeColor.g;
        b = modeColor.b;
        opacity = modeColor.a;
      }

      code += generateColorStyleDartCode(
        formatColorName(style.name, index), r, g, b, opacity
      );
    } else if (paint.type === "GRADIENT_LINEAR") {
      const stops = paint.gradientStops
        .map((stop) => {
          const { r, g, b } = stop.color;
          const a = 1;
          return `Color(0x${toHex(a)}${toHex(r)}${toHex(g)}${toHex(b)})`;
        })
        .join(", ");

      code += `  static const ${formatColorName(
        style.name, index
      )} = LinearGradient(colors: [${stops}]);\n\n`;
    }
  }

  code += "}\n";
  return code;
}

export async function generateColors(
  selectedModeIds?: string[]
): Promise<string> {
  try {
    const localColorStyles = await figma.getLocalPaintStylesAsync();

    if (localColorStyles.length === 0) {
      return "No defined colors";
    }

    let dartCode = initialize();

    if (selectedModeIds && selectedModeIds.length > 0) {
      const allModes = await collectColorModes();
      const modeMap = new Map(
        (allModes || []).map((m) => [m.modeId, m.name])
      );
      const useModeSuffix = selectedModeIds.length > 1;

      for (const modeId of selectedModeIds) {
        const modeName = modeMap.get(modeId) || modeId;
        dartCode += await generateColorsForMode(
          localColorStyles, modeId, modeName, useModeSuffix
        );
        dartCode += "\n";
      }
    } else {
      dartCode += "abstract class AppColors {\n";

      localColorStyles.forEach((style, index) => {
        const paint = style.paints[0];
        if (paint.type === "SOLID") {
          dartCode += generateColorStyleDartCode(
            formatColorName(style.name, index),
            paint.color.r, paint.color.g, paint.color.b,
            paint.opacity || 1
          );
        } else if (paint.type === "GRADIENT_LINEAR") {
          const stops = paint.gradientStops
            .map((stop) => {
              const { r, g, b } = stop.color;
              const a = 1;
              return `Color(0x${toHex(a)}${toHex(r)}${toHex(g)}${toHex(b)})`;
            })
            .join(", ");
          dartCode += `  static const ${formatColorName(
            style.name, index
          )} = LinearGradient(colors: [${stops}]);\n\n`;
        }
      });

      dartCode += "}\n";
    }

    return dartCode;
  } catch (error) {
    console.error("An error occurred:", error);
    return "";
  }
}

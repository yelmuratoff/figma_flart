import { initialize, formatEffectStyleName, formatClassName, toHex } from "../utils";

interface ModeInfo {
  modeId: string;
  name: string;
}

/**
 * Collects modes from effect styles that have bound variables.
 */
export async function collectEffectModes(): Promise<ModeInfo[] | null> {
  const styles = await figma.getLocalEffectStylesAsync();

  for (const style of styles) {
    for (const effect of style.effects) {
      const bound = (effect as any).boundVariables;
      if (!bound) continue;

      // Check any bound field (color, radius, spread, offsetX, offsetY)
      for (const field of Object.keys(bound)) {
        const alias = bound[field];
        if (!alias?.id) continue;

        const variable = await figma.variables.getVariableByIdAsync(alias.id);
        if (!variable) continue;

        const collection =
          await figma.variables.getVariableCollectionByIdAsync(
            variable.variableCollectionId
          );
        if (!collection || collection.modes.length <= 1) continue;

        return collection.modes.map((m) => ({
          modeId: m.modeId,
          name: m.name,
        }));
      }
    }
  }
  return null;
}

async function resolveEffectValueForMode(
  bound: any,
  field: string,
  modeId: string,
  fallback: number
): Promise<number> {
  const alias = bound?.[field];
  if (!alias?.id) return fallback;

  const variable = await figma.variables.getVariableByIdAsync(alias.id);
  if (!variable) return fallback;

  const val = variable.valuesByMode[modeId];
  return val !== undefined ? (val as number) : fallback;
}

async function resolveEffectColorForMode(
  bound: any,
  modeId: string,
  fallback: RGBA
): Promise<RGBA> {
  const alias = bound?.color;
  if (!alias?.id) return fallback;

  const variable = await figma.variables.getVariableByIdAsync(alias.id);
  if (!variable) return fallback;

  const val = variable.valuesByMode[modeId];
  if (!val || typeof val !== "object" || !("r" in val)) return fallback;

  return {
    r: (val as any).r,
    g: (val as any).g,
    b: (val as any).b,
    a: "a" in val ? (val as any).a : 1,
  };
}

async function generateEffectsForMode(
  styles: EffectStyle[],
  modeId: string,
  modeName: string,
  useModeSuffix: boolean
): Promise<string> {
  const suffix = useModeSuffix ? formatClassName(modeName) : "";
  let code = `abstract class App${suffix}EffectStyles {\n`;

  for (let sIndex = 0; sIndex < styles.length; sIndex++) {
    const style = styles[sIndex];
    const formattedStyleName = formatEffectStyleName(style.name, sIndex);

    for (let effectIndex = 0; effectIndex < style.effects.length; effectIndex++) {
      const effect = style.effects[effectIndex];
      const effectName = `${formattedStyleName}Effect${effectIndex}`;
      const bound = (effect as any).boundVariables;

      if (effect.type === "DROP_SHADOW" || effect.type === "INNER_SHADOW") {
        let { color, offset, radius, spread } = effect;

        if (bound) {
          color = await resolveEffectColorForMode(bound, modeId, color);
          radius = await resolveEffectValueForMode(bound, "radius", modeId, radius);
          spread = await resolveEffectValueForMode(bound, "spread", modeId, spread || 0);
          const ox = await resolveEffectValueForMode(bound, "offsetX", modeId, offset.x);
          const oy = await resolveEffectValueForMode(bound, "offsetY", modeId, offset.y);
          offset = { x: ox, y: oy };
        }

        const { r, g, b, a } = color;
        const colorCode = `Color(0x${toHex(a)}${toHex(r)}${toHex(g)}${toHex(b)})`;
        const offsetCode = `Offset(${offset.x}, ${offset.y})`;
        code += `  static const BoxShadow ${effectName} = BoxShadow(\n`;
        code += `    color: ${colorCode},\n`;
        code += `    offset: ${offsetCode},\n`;
        code += `    blurRadius: ${radius},\n`;
        code += `    spreadRadius: ${spread},\n`;
        code += `  );\n\n`;
      } else if (
        effect.type === "LAYER_BLUR" ||
        effect.type === "BACKGROUND_BLUR"
      ) {
        let { radius } = effect;
        if (bound) {
          radius = await resolveEffectValueForMode(bound, "radius", modeId, radius);
        }
        code += `  static const double ${effectName}BlurRadius = ${radius};\n\n`;
      }
    }
  }

  code += "}\n";
  return code;
}

export async function generateEffectStyles(
  selectedModeIds?: string[]
): Promise<string> {
  try {
    const localEffectStyles = await figma.getLocalEffectStylesAsync();

    if (localEffectStyles.length === 0) {
      return "No defined effect styles";
    }

    let dartCode = initialize();

    if (selectedModeIds && selectedModeIds.length > 0) {
      const allModes = await collectEffectModes();
      const modeMap = new Map(
        (allModes || []).map((m) => [m.modeId, m.name])
      );
      const useModeSuffix = selectedModeIds.length > 1;

      for (const modeId of selectedModeIds) {
        const modeName = modeMap.get(modeId) || modeId;
        dartCode += await generateEffectsForMode(
          localEffectStyles, modeId, modeName, useModeSuffix
        );
        dartCode += "\n";
      }
    } else {
      dartCode += "abstract class AppEffectStyles {\n";

      localEffectStyles.forEach((style, sIndex) => {
        const formattedStyleName = formatEffectStyleName(style.name, sIndex);

        style.effects.forEach((effect, effectIndex) => {
          const effectName = `${formattedStyleName}Effect${effectIndex}`;
          if (effect.type === "DROP_SHADOW" || effect.type === "INNER_SHADOW") {
            const { color, offset, radius, spread } = effect;
            const { r, g, b, a } = color;
            const colorCode = `Color(0x${toHex(a)}${toHex(r)}${toHex(g)}${toHex(b)})`;
            const offsetCode = `Offset(${offset.x}, ${offset.y})`;
            dartCode += `  static const BoxShadow ${effectName} = BoxShadow(\n`;
            dartCode += `    color: ${colorCode},\n`;
            dartCode += `    offset: ${offsetCode},\n`;
            dartCode += `    blurRadius: ${radius},\n`;
            dartCode += `    spreadRadius: ${spread},\n`;
            dartCode += `  );\n\n`;
          } else if (
            effect.type === "LAYER_BLUR" ||
            effect.type === "BACKGROUND_BLUR"
          ) {
            dartCode += `  static const double ${effectName}BlurRadius = ${effect.radius};\n\n`;
          }
        });
      });

      dartCode += "}\n";
    }

    return dartCode;
  } catch (error) {
    console.error("An error occurred:", error);
    return "";
  }
}

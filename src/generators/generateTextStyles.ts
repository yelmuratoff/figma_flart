import {
  initialize,
  formatStyleName,
  extractTextStyleProperties,
  generateTextStyleDartCode,
  formatClassName,
  inferFontStyleFromStyle,
  inferFontWeightFromStyle,
  mapTextDecorationToDart,
} from "../utils";

interface ModeInfo {
  modeId: string;
  name: string;
}

/**
 * Collects variable modes from text styles that have bound variables.
 * Returns null if no styles have bound variables (no modes).
 */
export async function collectTextStyleModes(): Promise<ModeInfo[] | null> {
  const textStyles = await figma.getLocalTextStylesAsync();

  for (const style of textStyles) {
    const bound = style.boundVariables;
    if (!bound) continue;

    const fields: (keyof typeof bound)[] = [
      "fontSize",
      "lineHeight",
      "letterSpacing",
      "fontFamily",
      "fontStyle",
      "fontWeight",
    ];

    for (const field of fields) {
      const alias = bound[field];
      if (!alias) continue;

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
  return null;
}

/**
 * Extracts text style properties for a specific mode.
 * Resolves bound variable values for the given modeId.
 */
async function extractTextStylePropertiesForMode(
  style: TextStyle,
  modeId: string
) {
  const bound = style.boundVariables || {};

  let fontSize = style.fontSize as number;
  if (bound.fontSize) {
    const variable = await figma.variables.getVariableByIdAsync(
      bound.fontSize.id
    );
    if (variable) {
      const val = variable.valuesByMode[modeId];
      if (val !== undefined) fontSize = val as number;
    }
  }

  let lineHeightValue: number | string = "null";
  if (bound.lineHeight) {
    const variable = await figma.variables.getVariableByIdAsync(
      bound.lineHeight.id
    );
    if (variable) {
      const val = variable.valuesByMode[modeId];
      if (val !== undefined) lineHeightValue = val as number;
    }
  } else if (
    style.lineHeight &&
    (style.lineHeight as any).unit !== "AUTO"
  ) {
    lineHeightValue = (style.lineHeight as any).value;
  }

  let letterSpacing = 0;
  if (bound.letterSpacing) {
    const variable = await figma.variables.getVariableByIdAsync(
      bound.letterSpacing.id
    );
    if (variable) {
      const val = variable.valuesByMode[modeId];
      if (val !== undefined) letterSpacing = val as number;
    }
  } else if (style.letterSpacing) {
    if ((style.letterSpacing as any).unit === "PIXELS") {
      letterSpacing = (style.letterSpacing as any).value;
    } else if ((style.letterSpacing as any).unit === "PERCENT") {
      letterSpacing =
        ((style.letterSpacing as any).value * fontSize) / 100;
    }
  }

  let fontFamily = style.fontName.family;
  if (bound.fontFamily) {
    const variable = await figma.variables.getVariableByIdAsync(
      bound.fontFamily.id
    );
    if (variable) {
      const val = variable.valuesByMode[modeId];
      if (val !== undefined) fontFamily = val as string;
    }
  }

  return {
    fontSize,
    fontStyle: inferFontStyleFromStyle(style.fontName.style),
    fontWeight: inferFontWeightFromStyle(style.fontName.style),
    letterSpacing: letterSpacing || 0,
    fontFamily,
    lineHeightValue,
    textDecoration: mapTextDecorationToDart(style.textDecoration),
  };
}

function generateTextStyleBlock(
  formattedStyleName: string,
  props: {
    fontSize: number;
    fontStyle: string;
    fontWeight: number;
    textDecoration: string;
    letterSpacing: number;
    fontFamily: string;
    lineHeightValue: number | string;
  },
  includeFontName: boolean,
  indent: string = "        "
): string {
  let code = `${indent}${formattedStyleName}: const TextStyle(\n`;
  code += `${indent}  fontSize: ${props.fontSize},\n`;
  code += `${indent}  fontWeight: FontWeight.w${props.fontWeight},\n`;

  if (includeFontName) {
    code += `${indent}  fontFamily: '${props.fontFamily}',\n`;
  }

  if (props.lineHeightValue !== "null") {
    const height =
      Math.round(
        ((props.lineHeightValue as number) / props.fontSize) * 100
      ) / 100;
    code += `${indent}  height: ${height},\n`;
  }

  if (props.letterSpacing != null && props.letterSpacing !== 0) {
    code += `${indent}  letterSpacing: ${props.letterSpacing},\n`;
  }
  code += `${indent}  fontStyle: ${props.fontStyle},\n`;
  code += `${indent}  decoration: ${props.textDecoration},\n`;
  code += `${indent}),\n`;

  return code;
}

// ── Main function ──

async function generateThemeExtensionForMode(
  textStyles: TextStyle[],
  includeFontName: boolean,
  modeId: string,
  modeName: string,
  useModeSuffix: boolean
): Promise<string> {
  const suffix = useModeSuffix ? formatClassName(modeName) : "";
  const className = `App${suffix}TextTheme`;
  let code = "";

  code += `@immutable\nclass ${className} extends ThemeExtension<${className}> {\n`;

  textStyles.forEach((style, index) => {
    const name = formatStyleName(style.name, index);
    code += `  final TextStyle? ${name};\n`;
  });

  code += `\n   const ${className}({\n`;
  textStyles.forEach((style, index) => {
    code += `    this.${formatStyleName(style.name, index)},\n`;
  });
  code += "  });\n\n";

  code += `  const ${className}.fallback()\n      : this(\n`;
  for (let index = 0; index < textStyles.length; index++) {
    const style = textStyles[index];
    const props = await extractTextStylePropertiesForMode(style, modeId);
    code += generateTextStyleBlock(
      formatStyleName(style.name, index),
      props,
      includeFontName
    );
  }
  code += "      );\n\n";

  code += `  @override\n  ${className} copyWith({\n`;
  textStyles.forEach((style, index) => {
    code += `    TextStyle? ${formatStyleName(style.name, index)},\n`;
  });
  code += `  }) {\n    return ${className}(\n`;
  textStyles.forEach((style, index) => {
    const n = formatStyleName(style.name, index);
    code += `      ${n}: ${n} ?? this.${n},\n`;
  });
  code += "    );\n  }\n\n";

  code += `  @override\n  ${className} lerp(${className}? other, double t) {\n`;
  code += `    if (other is! ${className}) return this;\n`;
  code += `    return ${className}(\n`;
  textStyles.forEach((style, index) => {
    const n = formatStyleName(style.name, index);
    code += `      ${n}: TextStyle.lerp(${n}, other.${n}, t),\n`;
  });
  code += "    );\n  }\n";
  code += "}\n";

  return code;
}

async function generateAbstractClassForMode(
  textStyles: TextStyle[],
  includeFontName: boolean,
  modeId: string,
  modeName: string,
  useModeSuffix: boolean
): Promise<string> {
  const suffix = useModeSuffix ? formatClassName(modeName) : "";
  let code = `abstract class App${suffix}TextStyles {\n`;

  for (let index = 0; index < textStyles.length; index++) {
    const style = textStyles[index];
    const props = await extractTextStylePropertiesForMode(style, modeId);
    code += generateTextStyleDartCode(
      formatStyleName(style.name, index),
      props,
      includeFontName
    );
  }

  code += `  \n  static List<Map<String, dynamic>> get map => [\n`;
  textStyles.forEach((style, index) => {
    const n = formatStyleName(style.name, index);
    code += `    {'${n}': ${n}},\n`;
  });
  code += "  ];\n";
  code += "}\n";

  return code;
}

export async function generateTextStyles(
  useThemeExtensions: boolean,
  includeFontName: boolean,
  selectedModeIds?: string[]
): Promise<string> {
  try {
    const textStyles = await figma.getLocalTextStylesAsync();
    if (textStyles.length === 0) {
      return "No defined textstyles";
    }

    let dartCode = initialize();

    if (selectedModeIds && selectedModeIds.length > 0) {
      // Resolve mode names
      const allModes = await collectTextStyleModes();
      const modeMap = new Map(
        (allModes || []).map((m) => [m.modeId, m.name])
      );

      const useModeSuffix = selectedModeIds.length > 1;

      for (const modeId of selectedModeIds) {
        const modeName = modeMap.get(modeId) || modeId;

        if (useThemeExtensions) {
          dartCode += await generateThemeExtensionForMode(
            textStyles, includeFontName, modeId, modeName, useModeSuffix
          );
        } else {
          dartCode += await generateAbstractClassForMode(
            textStyles, includeFontName, modeId, modeName, useModeSuffix
          );
        }
        dartCode += "\n";
      }
    } else {
      // No modes — original behavior
      if (useThemeExtensions) {
        const className = "AppTextTheme";
        dartCode += `@immutable\nclass ${className} extends ThemeExtension<${className}> {\n`;

        textStyles.forEach((style, index) => {
          dartCode += `  final TextStyle? ${formatStyleName(style.name, index)};\n`;
        });

        dartCode += `\n   const ${className}({\n`;
        textStyles.forEach((style, index) => {
          dartCode += `    this.${formatStyleName(style.name, index)},\n`;
        });
        dartCode += "  });\n\n";

        dartCode += `  const ${className}.fallback()\n      : this(\n`;
        textStyles.forEach((style, index) => {
          const props = extractTextStyleProperties(style);
          dartCode += generateTextStyleBlock(
            formatStyleName(style.name, index), props, includeFontName
          );
        });
        dartCode += "      );\n\n";

        dartCode += `  @override\n  ${className} copyWith({\n`;
        textStyles.forEach((style, index) => {
          dartCode += `    TextStyle? ${formatStyleName(style.name, index)},\n`;
        });
        dartCode += `  }) {\n    return ${className}(\n`;
        textStyles.forEach((style, index) => {
          const n = formatStyleName(style.name, index);
          dartCode += `      ${n}: ${n} ?? this.${n},\n`;
        });
        dartCode += "    );\n  }\n\n";

        dartCode += `  @override\n  ${className} lerp(${className}? other, double t) {\n`;
        dartCode += `    if (other is! ${className}) return this;\n`;
        dartCode += `    return ${className}(\n`;
        textStyles.forEach((style, index) => {
          const n = formatStyleName(style.name, index);
          dartCode += `      ${n}: TextStyle.lerp(${n}, other.${n}, t),\n`;
        });
        dartCode += "    );\n  }\n";
        dartCode += "}\n";
      } else {
        dartCode += "abstract class AppTextStyles {\n";
        textStyles.forEach((style, index) => {
          const props = extractTextStyleProperties(style);
          dartCode += generateTextStyleDartCode(
            formatStyleName(style.name, index), props, includeFontName
          );
        });
        dartCode += `  \n  static List<Map<String, dynamic>> get map => [\n`;
        textStyles.forEach((style, index) => {
          const n = formatStyleName(style.name, index);
          dartCode += `    {'${n}': ${n}},\n`;
        });
        dartCode += "  ];\n";
        dartCode += "}\n";
      }
    }

    return dartCode;
  } catch (error) {
    console.error("An error occurred:", error);
    return "";
  }
}

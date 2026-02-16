import {
  initialize,
  formatStyleName,
  extractTextStyleProperties,
  generateTextStyleDartCode,
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

export async function generateTextStyles(
  useThemeExtensions: boolean,
  includeFontName: boolean,
  selectedModeId?: string
): Promise<string> {
  try {
    const textStyles = await figma.getLocalTextStylesAsync();
    if (textStyles.length === 0) {
      return "No defined textstyles";
    }

    let dartCode = initialize();

    if (useThemeExtensions) {
      const className = "AppTextTheme";

      // Generate fields
      dartCode +=
        `@immutable\nclass ${className} extends ThemeExtension<${className}> {\n`;

      textStyles.forEach((style, index) => {
        const name = formatStyleName(style.name, index);
        dartCode += `  final TextStyle? ${name};\n`;
      });

      // Constructor
      dartCode += `\n   const ${className}({\n`;
      textStyles.forEach((style, index) => {
        const name = formatStyleName(style.name, index);
        dartCode += `    this.${name},\n`;
      });
      dartCode += "  });\n\n";

      // Fallback constructor
      dartCode += `  const ${className}.fallback()\n      : this(\n`;
      for (let index = 0; index < textStyles.length; index++) {
        const style = textStyles[index];
        const formattedStyleName = formatStyleName(style.name, index);

        const props = selectedModeId
          ? await extractTextStylePropertiesForMode(style, selectedModeId)
          : extractTextStyleProperties(style);

        dartCode += generateTextStyleBlock(
          formattedStyleName,
          props,
          includeFontName
        );
      }
      dartCode += "      );\n\n";

      // copyWith
      dartCode += `  @override\n  ${className} copyWith({\n`;
      textStyles.forEach((style, index) => {
        const name = formatStyleName(style.name, index);
        dartCode += `    TextStyle? ${name},\n`;
      });
      dartCode += `  }) {\n    return ${className}(\n`;
      textStyles.forEach((style, index) => {
        const name = formatStyleName(style.name, index);
        dartCode += `      ${name}: ${name} ?? this.${name},\n`;
      });
      dartCode += "    );\n  }\n\n";

      // lerp
      dartCode +=
        `  @override\n  ${className} lerp(${className}? other, double t) {\n`;
      dartCode += `    if (other is! ${className}) return this;\n`;
      dartCode += `    return ${className}(\n`;
      textStyles.forEach((style, index) => {
        const name = formatStyleName(style.name, index);
        dartCode += `      ${name}: TextStyle.lerp(${name}, other.${name}, t),\n`;
      });
      dartCode += "    );\n  }\n";
      dartCode += "}\n";
    } else {
      dartCode += "abstract class AppTextStyles {\n";

      for (let index = 0; index < textStyles.length; index++) {
        const style = textStyles[index];
        const formattedStyleName = formatStyleName(style.name, index);

        const props = selectedModeId
          ? await extractTextStylePropertiesForMode(style, selectedModeId)
          : extractTextStyleProperties(style);

        dartCode += generateTextStyleDartCode(
          formattedStyleName,
          props,
          includeFontName
        );
      }

      dartCode += `  \n  static List<Map<String, dynamic>> get map => [\n`;
      textStyles.forEach((style, index) => {
        const formattedStyleName = formatStyleName(style.name, index);
        dartCode += `    {'${formattedStyleName}': ${formattedStyleName}},\n`;
      });
      dartCode += "  ];\n";
      dartCode += "}\n";
    }

    return dartCode;
  } catch (error) {
    console.error("An error occurred:", error);
    return "";
  }
}

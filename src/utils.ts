function generateTextStyleDartCode(
  styleName: string,
  {
    fontSize,
    fontStyle,
    fontWeight,
    textDecoration,
    letterSpacing,
    fontFamily,
    lineHeightValue,
  }: any,
  includeFontName: boolean
): string {
  let code = `  static const TextStyle ${styleName} = TextStyle(\n`;
  code += `    fontSize: ${fontSize},\n`;
  code += `    fontWeight: FontWeight.w${fontWeight},\n`;

  if (includeFontName) {
    code += `    fontFamily: '${fontFamily}',\n`;
  }

  if (lineHeightValue !== "null") {
    const height = Math.round((lineHeightValue / fontSize) * 100) / 100;
    code += `    height: ${height},\n`;
  }

  if (
    letterSpacing != null &&
    letterSpacing !== "null" &&
    letterSpacing !== 0
  ) {
    console.log("letterSpacing", letterSpacing, fontSize);
    //  const roundedLetterSpacing = Number(letterSpacing.toFixed(1));
    code += `    letterSpacing: ${letterSpacing},\n`;
  }
  code += `    fontStyle: ${fontStyle},\n`;
  code += `    decoration: ${textDecoration},\n`;
  code += `  );\n\n`;

  return code;
}

function generateColorStyleDartCode(
  styleName: string,
  r: number,
  g: number,
  b: number,
  opacity: number = 1
): string {
  // Convert color channels and opacity to hex format
  const a = padStart(Math.floor(opacity * 255).toString(16), 2, "0");
  const rHex = padStart(Math.floor(r * 255).toString(16), 2, "0");
  const gHex = padStart(Math.floor(g * 255).toString(16), 2, "0");
  const bHex = padStart(Math.floor(b * 255).toString(16), 2, "0");

  // Generate Dart code
  let code = `  static const Color ${styleName} = Color(0x${a}${rHex}${gHex}${bHex});\n\n`;

  return code;
}

function inferFontWeightFromStyle(fontStyle: string): number {
  if (fontStyle === "Black") return 900;
  if (fontStyle === "Extra Bold" || fontStyle === "Heavy") return 800;
  if (fontStyle === "Bold") return 700;
  if (fontStyle === "Semi Bold" || fontStyle === "DemiBold") return 600;
  if (fontStyle === "Medium") return 500;
  if (fontStyle === "Regular" || fontStyle === "Normal") return 400;
  if (fontStyle === "Light") return 300;
  if (fontStyle === "Extra Light" || fontStyle === "Ultra Light") return 200;
  if (fontStyle === "Thin" || fontStyle === "Hairline") return 100;
  return 400; // Default weight
}

function inferFontStyleFromStyle(fontStyle: string): string {
  if (fontStyle.includes("Italic")) return "FontStyle.italic";
  if (fontStyle.includes("Oblique")) return "FontStyle.italic";
  return "FontStyle.normal";
}

function mapTextDecorationToDart(decoration: string): string {
  const map: Record<string, string> = {
    none: "TextDecoration.none",
    underline: "TextDecoration.underline",
    overline: "TextDecoration.overline",
    "line-through": "TextDecoration.lineThrough",
  };
  return map.hasOwnProperty(decoration.toLowerCase())
    ? map[decoration.toLowerCase()]
    : "TextDecoration.none";
}

function formatColorName(name: string, index: number): string {
  if (!name) return `color${index}`;
  const words = name
    .replace(/[^a-zA-Z0-9 ]/g, " ")
    .trim()
    .split(/\s+/);
  return words
    .map((word, index) =>
      index === 0 ? word.toLowerCase() : capitalizeFirstLetter(word)
    )
    .join("");
}

function formatEffectStyleName(name: string, index: number): string {
  if (!name) return `effectStyle${index}`;
  const words = name
    .replace(/[^a-zA-Z0-9 ]/g, " ")
    .trim()
    .split(/\s+/);
  return words
    .map((word, index) =>
      index === 0 ? word.toLowerCase() : capitalizeFirstLetter(word)
    )
    .join("");
}

function extractTextStyleProperties(style: any) {
  let letterSpacing = 0;
  console.log("style", style);
  const fontSize = style.fontSize;

  if (style.letterSpacing) {
    if (style.letterSpacing.unit === "PIXELS") {
      letterSpacing = style.letterSpacing.value;
    } else if (style.letterSpacing.unit === "PERCENT") {
      letterSpacing = (style.letterSpacing.value * fontSize) / 100;
    }
  }

  return {
    fontSize: style.fontSize,
    fontStyle: inferFontStyleFromStyle(style.fontName.style),
    fontWeight: inferFontWeightFromStyle(style.fontName.style),
    decoration: style.textDecoration,
    letterSpacing: letterSpacing || 0,
    fontFamily: style.fontName.family,
    lineHeightValue:
      style.lineHeight?.unit !== "AUTO" ? style.lineHeight.value : "null",
    textDecoration: mapTextDecorationToDart(style.textDecoration),
  };
}

function formatStyleName(name: string, index: number): string {
  if (!name) return `textStyle${index}`;
  const words = name
    .replace(/[^a-zA-Z0-9 ]/g, " ")
    .trim()
    .split(/\s+/);
  return words
    .map((word, index) =>
      index === 0 ? word.toLowerCase() : capitalizeFirstLetter(word)
    )
    .join("");
}

function toHex(channel: number): string {
  return padStart(Math.floor(channel * 255).toString(16), 2, "0");
}

function padStart(
  str: string,
  maxLength: number,
  fillString: string = " "
): string {
  if (str.length >= maxLength) {
    return str;
  }
  return Array(maxLength - str.length + 1).join(fillString) + str;
}

function formatVariableName(name: string): string {
  const formatted = name.replace(/[^a-zA-Z0-9]+(.)/g, (m, chr) =>
    chr.toUpperCase()
  );
  return formatted.charAt(0).toLowerCase() + formatted.slice(1);
}

function removeSpacesAndDigits(text: string): string {
  return text.replace(/[0-9\s]/g, "");
}

function formatVariableNameWMode(
  mode: string,
  hasMode: boolean,
  name: string
): string {
  if (!hasMode) {
    return formatVariableName(name);
  }
  return removeSpacesAndDigits(
    `${mode.toLowerCase()}${capitalize(
      name.replace(/[^a-zA-Z0-9]+(.)/g, (m, chr) => chr.toUpperCase())
    )}`
  );
}

function formatClassName(name: string): string {
  return removeSpacesAndDigits(
    name
      .replace(/[^a-zA-Z0-9]+(.)/g, (m, chr) => chr.toUpperCase())
      .replace(/^(.)/, (chr) => chr.toUpperCase())
  );
}

function capitalizeFirstLetter(string: string): string {
  return string.charAt(0).toUpperCase() + string.slice(1);
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function convertToCamelCase(name: string): string {
  return (
    name.charAt(0).toUpperCase() +
    name.slice(1).replace(/\/(.)/g, (_, letter) => letter.toUpperCase())
  );
}

function getDartValue(variable: any, mode: any): string {
  const valueForMode = variable?.values?.find(
    (v: any) => v.modeId === mode.modeId
  );
  if (variable.resolvedType === "COLOR" && valueForMode) {
    const color = valueForMode;
    return `Color(0x${toHex(color.r)}${toHex(color.g)}${toHex(color.b)})`;
  } else if (variable.resolvedType === "FLOAT" && valueForMode) {
    return valueForMode.toString();
  }
  return "null";
}

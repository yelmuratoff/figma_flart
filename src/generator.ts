/// <---------------------- UTILS ---------------------->

// Infers Dart fontWeight from Figma fontStyle
function inferFontWeightFromStyle(fontStyle: string): number {
  if (fontStyle.includes("Bold")) return 700;
  if (fontStyle.includes("Medium")) return 500;
  return 400; // Default weight
}

// Infers Dart fontStyle from Figma fontStyle
function inferFontStyleFromStyle(fontStyle: string): string {
  if (fontStyle.includes("Italic")) return "FontStyle.italic";
  if (fontStyle.includes("Oblique")) return "FontStyle.italic"; // Dart doesn't support 'oblique'
  return "FontStyle.normal"; // Default style
}

// Maps Figma text decorations to Dart's TextDecoration enum
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
  if (!name) return `color${index}`; // or any default name you prefer
  const words = name
    .replace(/[^a-zA-Z0-9 ]/g, " ") // Remove all non-alphanumeric characters and replace with space
    .trim() // Remove leading and trailing spaces
    .split(/\s+/); // Split by one or more spaces
  return words
    .map((word, index) =>
      index === 0 ? word.toLowerCase() : capitalizeFirstLetter(word)
    )
    .join("");
}

function formatEffectStyleName(name: string, index: number): string {
  if (!name) return `effectStyle${index}`;
  const words = name
    .replace(/[^a-zA-Z0-9 ]/g, " ") // Remove all non-alphanumeric characters and replace with space
    .trim() // Remove leading and trailing spaces
    .split(/\s+/); // Split by one or more spaces
  return words
    .map((word, index) =>
      index === 0 ? word.toLowerCase() : capitalizeFirstLetter(word)
    )
    .join("");
}

function extractTextStyleProperties(style: any) {
  let letterSpacing = 0; // Default value
  const fontSize = style.fontSize;

  // Check if letterSpacing is in pixels or percent and convert accordingly
  if (style.letterSpacing) {
    if (style.letterSpacing.unit === "PIXELS") {
      letterSpacing = style.letterSpacing.value; // Use pixel value directly
    } else if (style.letterSpacing.unit === "PERCENT") {
      // Convert percentage to pixels based on the fontSize
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

// Format style name
function formatStyleName(name: string, index: number): string {
  if (!name) return `textStyle${index}`;
  const words = name
    .replace(/[^a-zA-Z0-9 ]/g, " ") // Remove all non-alphanumeric characters and replace with space
    .trim() // Remove leading and trailing spaces
    .split(/\s+/); // Split by one or more spaces
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

/// <---------------------- GENERATOR ---------------------->

async function generateTextStyles(
  useThemeExtensions: boolean,
  includeFontName: boolean
): Promise<string> {
  try {
    const textStyles = await figma.getLocalTextStylesAsync();
    if (textStyles.length === 0) {
      return "No defined textstyles";
    }
    let dartCode = "import 'package:flutter/material.dart';\n\n";

    if (useThemeExtensions) {
      // For AppTextTheme
      dartCode +=
        "@immutable\nclass AppTextTheme extends ThemeExtension<AppTextTheme> {\n";

      // Generate fields
      textStyles.forEach((style, index) => {
        const formattedStyleName = formatStyleName(style.name, index);
        dartCode += `  final TextStyle? ${formattedStyleName};\n`;
      });

      // Generate constructor
      dartCode += "\n   const AppTextTheme({\n";
      textStyles.forEach((style, index) => {
        const formattedStyleName = formatStyleName(style.name, index);
        dartCode += `    this.${formattedStyleName},\n`;
      });
      dartCode += "  });\n\n";

      // Generate fallback constructor
      dartCode += "  const AppTextTheme.fallback()\n      : this(\n";
      textStyles.forEach((style, index) => {
        const {
          fontSize,
          fontStyle,
          fontWeight,
          textDecoration,
          letterSpacing,
          fontFamily,
          lineHeightValue,
        } = extractTextStyleProperties(style);
        const formattedStyleName = formatStyleName(style.name, index);

        dartCode += `        ${formattedStyleName}: const TextStyle(\n`;
        dartCode += `          fontSize: ${fontSize},\n`;
        dartCode += `          fontWeight: FontWeight.w${fontWeight},\n`;

        if (includeFontName) {
          dartCode += `          fontFamily: "${fontFamily}",\n`;
        }

        if (lineHeightValue !== "null") {
          const height = Math.round((lineHeightValue / fontSize) * 100) / 100;
          dartCode += `          height: ${height},\n`;
        }

        if (letterSpacing != null && letterSpacing !== 0) {
          const roundedLetterSpacing =
            Math.round(((letterSpacing * fontSize) / 100) * 100) / 100;
          dartCode += `          letterSpacing: ${roundedLetterSpacing},\n`;
        }
        dartCode += `          fontStyle: ${fontStyle},\n`;
        dartCode += `          decoration: ${textDecoration},\n`;
        dartCode += `        ),\n`;
      });
      dartCode += "      );\n\n";

      // Generate copyWith method
      dartCode += "  @override\n  AppTextTheme copyWith({\n";
      textStyles.forEach((style, index) => {
        const formattedStyleName = formatStyleName(style.name, index);
        dartCode += `    TextStyle? ${formattedStyleName},\n`;
      });
      dartCode += "  }) {\n    return AppTextTheme(\n";
      textStyles.forEach((style, index) => {
        const formattedStyleName = formatStyleName(style.name, index);
        dartCode += `      ${formattedStyleName}: ${formattedStyleName} ?? this.${formattedStyleName},\n`;
      });
      dartCode += "    );\n  }\n\n";

      // Generate lerp method
      dartCode +=
        "  @override\n  AppTextTheme lerp(AppTextTheme? other, double t) {\n";
      dartCode += "    if (other is! AppTextTheme) return this;\n";
      dartCode += "    return AppTextTheme(\n";
      textStyles.forEach((style, index) => {
        const formattedStyleName = formatStyleName(style.name, index);
        dartCode += `      ${formattedStyleName}: TextStyle.lerp(${formattedStyleName}, other.${formattedStyleName}, t),\n`;
      });
      dartCode += "    );\n  }\n";
    } else {
      // Original TextStyles class
      dartCode += "abstract class AppTextStyles {\n";
      textStyles.forEach((style, index) => {
        const formattedStyleName = formatStyleName(style.name, index);
        const {
          fontSize,
          fontStyle,
          fontWeight,
          textDecoration,
          letterSpacing,
          fontFamily,
          lineHeightValue,
        } = extractTextStyleProperties(style);

        dartCode += generateTextStyleDartCode(
          formattedStyleName,
          {
            fontSize,
            fontStyle,
            fontWeight,
            textDecoration,
            letterSpacing,
            fontFamily,
            lineHeightValue,
          },
          includeFontName
        );
      });
    }

    dartCode += "}\n";
    return dartCode;
  } catch (error) {
    console.error("An error occurred:", error);
    return "";
  }
}

async function generateColors(): Promise<string> {
  try {
    const localColorStyles = await figma.getLocalPaintStylesAsync();

    if (localColorStyles.length === 0) {
      return "No defined colors";
    }

    let dartCode = "import 'package:flutter/material.dart';\n\n";
    dartCode += "abstract class AppColors {\n";

    localColorStyles.forEach((style, index) => {
      // Changed to forEach to get index
      const paint = style.paints[0]; // assuming the first paint is what you want
      if (paint.type === "SOLID") {
        const r = paint.color.r;
        const g = paint.color.g;
        const b = paint.color.b;
        const opacity = paint.opacity || 1;

        dartCode += generateColorStyleDartCode(
          formatColorName(style.name, index),
          r,
          g,
          b,
          opacity
        ); // Passed index
      } else if (paint.type === "GRADIENT_LINEAR") {
        // Assuming stops is an array of color stop objects containing color and position
        const stops = paint.gradientStops
          .map((stop) => {
            const { r, g, b } = stop.color;
            const a = 1;
            return `Color(0x${toHex(a)}${toHex(r)}${toHex(g)}${toHex(b)})`;
          })
          .join(", ");

        dartCode += `  static const ${formatColorName(
          style.name,
          index
        )} = LinearGradient(colors: [${stops}]);\n\n`; // Passed index
      }
    });

    dartCode += "}\n";
    return dartCode;
  } catch (error) {
    console.error("An error occurred:", error);
    return "";
  }
}

async function generateEffectStyles(): Promise<string> {
  try {
    const localEffectStyles = await figma.getLocalEffectStylesAsync();

    if (localEffectStyles.length === 0) {
      return "No defined effect styles";
    }

    let dartCode = "import 'package:flutter/material.dart';\n\n";
    dartCode += "abstract class AppEffectStyles {\n";

    localEffectStyles.forEach((style, index) => {
      const formattedStyleName = formatEffectStyleName(style.name, index);
      const effects = style.effects; // Array of effects

      effects.forEach((effect, effectIndex) => {
        const effectName = `${formattedStyleName}Effect${effectIndex}`;
        if (effect.type === "DROP_SHADOW" || effect.type === "INNER_SHADOW") {
          const { color, offset, radius, spread } = effect;
          const { r, g, b, a } = color;
          const colorCode = `Color(0x${toHex(a)}${toHex(r)}${toHex(g)}${toHex(
            b
          )})`;
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
          const { radius } = effect;
          dartCode += `  static const double ${effectName}BlurRadius = ${radius};\n\n`;
        }
      });
    });

    dartCode += "}\n";
    return dartCode;
  } catch (error) {
    console.error("An error occurred:", error);
    return "";
  }
}

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
    code += `    fontFamily: "${fontFamily}",\n`;
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
    const roundedLetterSpacing =
      Math.round(((letterSpacing * fontSize) / 100) * 100) / 100;
    code += `    letterSpacing: ${roundedLetterSpacing},\n`;
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

async function generateVariables(useThemeExtensions: boolean): Promise<string> {
  const collections = await figma.variables.getLocalVariableCollectionsAsync();
  let dartCode =
    "// This file is generated by Flart plugin.\n// Please, do not edit this file.\n\nimport 'dart:ui';\n";
  dartCode += "import 'package:flutter/material.dart';\n\n";

  // Проходим по каждой коллекции переменных
  for (const collection of collections) {
    const className = formatClassName(collection.name);
    const hasVariables = collection.variableIds.length > 0;

    if (useThemeExtensions && hasVariables) {
      // Интерфейс для ThemeExtension
      dartCode += `abstract interface class IApp${className} extends ThemeExtension<IApp${className}> {\n`;

      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        const variableName = formatVariableName(variable?.name ?? "");

        if (variable?.resolvedType === "COLOR") {
          dartCode += `  abstract final Color ${variableName};\n`;
        } else if (variable?.resolvedType === "FLOAT") {
          dartCode += `  abstract final double ${variableName};\n`;
        } else {
          dartCode += `  abstract final dynamic ${variableName};\n`; // Используем dynamic для других типов
        }
      }
      dartCode += "}\n\n";

      // Класс Colors с реализацией
      dartCode += `@immutable\nfinal class App${className} implements IApp${className} {\n`;
      dartCode += `  const App${className}({\n`;

      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        if (!variable) continue;
        const variableName = formatVariableName(variable.name);

        dartCode += `    required this.${variableName},\n`;
      }

      dartCode += "  });\n\n";

      // Поля для переменных
      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        if (!variable) continue;
        const variableName = formatVariableName(variable.name);

        dartCode += `  @override\n  final ${
          variable?.resolvedType === "COLOR"
            ? "Color"
            : variable?.resolvedType === "FLOAT"
            ? "double"
            : "dynamic"
        } ${variableName};\n`;
      }

      // copyWith метод
      dartCode += `\n  @override\n App${className} copyWith({\n`;
      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        if (!variable) continue;
        const variableName = formatVariableName(variable.name);

        dartCode += `    ${
          variable?.resolvedType === "COLOR"
            ? "Color"
            : variable?.resolvedType === "FLOAT"
            ? "double"
            : "dynamic"
        }? ${variableName},\n`;
      }
      dartCode += `  }) => App${className}(\n`;
      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        if (!variable) continue;
        const variableName = formatVariableName(variable.name);

        dartCode += `    ${variableName}: ${variableName} ?? this.${variableName},\n`;
      }
      dartCode += "  );\n\n";

      // lerp метод
      dartCode += `\n  @override\n  App${className} lerp(App${className}? other, double t) {\n`;
      dartCode += "    if (other == null) return this;\n";
      dartCode += `    return App${className}(\n`;

      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        if (!variable) continue;
        const variableName = formatVariableName(variable.name);

        if (variable?.resolvedType === "COLOR") {
          dartCode += `      ${variableName}: Color.lerp(${variableName}, other.${variableName}, t)!,\n`;
        } else if (variable?.resolvedType === "FLOAT") {
          dartCode += `      ${variableName}: lerpDouble(${variableName}, other.${variableName}, t)!,\n`;
        } else {
          dartCode += `      ${variableName}: other.${variableName} ?? this.${variableName},\n`; // Dynamic для lerp
        }
      }
      dartCode += "    );\n  }\n";

      // equals метод
      dartCode += `\n  @override\n  bool operator ==(Object other) {\n`;
      dartCode += `    if (identical(this, other)) return true;\n`;
      dartCode += `    return other is App${className} &&\n`;
      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        if (!variable) continue;
        const variableName = formatVariableName(variable.name);

        dartCode += `      other.${variableName} == ${variableName} &&\n`;
      }
      dartCode = dartCode.slice(0, -4); // Remove the last '&&\n'
      dartCode += ";\n  }\n";

      // hashCode метод
      dartCode += `\n  @override\n  int get hashCode =>\n`;
      dartCode += "    Object.hashAll([\n";
      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        if (!variable) continue;
        const variableName = formatVariableName(variable.name);

        dartCode += `      ${variableName},\n`;
      }
      dartCode += "    ]);\n\n";

      // Метод type
      dartCode += `  @override\n  Object get type => runtimeType;\n\n`;

      // Генерация значений для разных модов с учетом алиасов
      for (const mode of collection.modes) {
        dartCode += `  static IApp${className} get ${mode.name.toLowerCase()}${className} => const App${className}(\n`;

        for (const variableId of collection.variableIds) {
          const variable = await figma.variables.getVariableByIdAsync(
            variableId
          );
          const value = variable?.valuesByMode[mode.modeId];
          const variableName = formatVariableName(variable?.name ?? "");

          if (typeof value === "object" && value !== null && "type" in value) {
            const aliasVariable = await figma.variables.getVariableByIdAsync(
              (value as VariableAlias).id
            );
            const aliasValue = aliasVariable?.valuesByMode["1:0"];

            if (aliasVariable?.resolvedType === "COLOR") {
              const asColor = aliasValue as RGBA;
              const colorCode = `Color(0x${toHex(asColor.a)}${toHex(
                asColor.r
              )}${toHex(asColor.g)}${toHex(asColor.b)})`;
              dartCode += `    ${variableName}: ${colorCode},\n`;
            } else {
              dartCode += `    ${variableName}: ${aliasValue},\n`;
            }
          } else if (
            typeof value === "object" &&
            value !== null &&
            "r" in value
          ) {
            const asColor = value as RGBA;
            const colorCode = `Color(0x${toHex(asColor.a)}${toHex(
              asColor.r
            )}${toHex(asColor.g)}${toHex(asColor.b)})`;
            dartCode += `    ${variableName}: ${colorCode},\n`;
          } else {
            dartCode += `    ${variableName}: ${value},\n`;
          }
        }
        dartCode += ");\n\n";
      }

      dartCode += "}\n";
    } else {
      let classContent = `final class App${capitalizeFirstLetter(
        collection.name
      )} {\n  const App${capitalizeFirstLetter(collection.name)}._();\n\n`;

      for (const mode of collection.modes) {
        for (const variableId of collection.variableIds) {
          const variable = await figma.variables.getVariableByIdAsync(
            variableId
          );
          const value = variable?.valuesByMode[mode.modeId];

          if (typeof value === "object" && value !== null && "type" in value) {
            const aliasVariable = await figma.variables.getVariableByIdAsync(
              (value as VariableAlias).id
            );
            const aliasValue = aliasVariable?.valuesByMode["1:0"];

            if (aliasVariable?.resolvedType === "COLOR") {
              const asColor = aliasValue as RGBA;
              const colorCode = `Color(0x${toHex(asColor.a)}${toHex(
                asColor.r
              )}${toHex(asColor.g)}${toHex(asColor.b)})`;
              classContent += `  static const Color ${formatVariableNameWMode(
                mode.name,
                variable?.name ?? "null"
              )} = ${colorCode};\n`;
            } else {
              classContent += `  static const dynamic ${formatVariableNameWMode(
                mode.name,
                variable?.name ?? "null"
              )} = ${aliasValue};\n`;
            }
          } else if (
            typeof value === "object" &&
            value !== null &&
            "r" in value &&
            "g" in value &&
            "b" in value
          ) {
            const asColor = "a" in value ? (value as RGBA) : { ...value, a: 1 };
            const colorCode = `Color(0x${toHex(asColor.a)}${toHex(
              asColor.r
            )}${toHex(asColor.g)}${toHex(asColor.b)})`;
            classContent += `  static const Color ${formatVariableNameWMode(
              mode.name,
              variable?.name ?? "null"
            )} = ${colorCode};\n`;
          } else {
            classContent += `  static const dynamic ${formatVariableNameWMode(
              mode.name,
              variable?.name ?? "null"
            )} = ${value};\n`;
          }
        }
      }

      classContent += `}\n\n`;
      dartCode += classContent;
    }
  }

  return dartCode;
}

// Функции вспомогательные
function formatVariableName(name: string): string {
  return `${name.replace(/[^a-zA-Z0-9]+(.)/g, (m, chr) => chr.toUpperCase())}`;
}

// Функции вспомогательные
function formatVariableNameWMode(mode: string, name: string): string {
  return `${mode.toLowerCase()}${capitalize(
    name.replace(/[^a-zA-Z0-9]+(.)/g, (m, chr) => chr.toUpperCase())
  )}`;
}

// Функция для форматирования имени класса
function formatClassName(name: string): string {
  return name
    .replace(/[^a-zA-Z0-9]+(.)/g, (m, chr) => chr.toUpperCase())
    .replace(/^(.)/, (chr) => chr.toUpperCase());
}

function capitalizeFirstLetter(string: string): string {
  return string.charAt(0).toUpperCase() + string.slice(1);
}

// // Форматирование переменной в camelCase
// function formatVariableName(mode: string, name: string): string {
//   var result = mode.toLowerCase() + capitalize(name);
//   return result.replace(/[^a-zA-Z0-9]+(.)/g, (m, chr) => chr.toUpperCase());
// }

// Конвертация первой буквы в верхний регистр
function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

// Конвертация имени переменной в camelCase
function convertToCamelCase(name: string): string {
  return (
    name.charAt(0).toUpperCase() +
    name.slice(1).replace(/\/(.)/g, (_, letter) => letter.toUpperCase())
  );
}

// Получение значения переменной для Dart (например, Color или число)
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

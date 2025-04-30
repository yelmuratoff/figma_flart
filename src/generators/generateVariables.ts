async function generateVariables(useThemeExtensions: boolean): Promise<string> {
  const collections = await figma.variables.getLocalVariableCollectionsAsync();
  let dartCode = initialize();

  // Проходим по каждой коллекции переменных
  for (const collection of collections) {
    const className = formatClassName(collection.name);
    const hasVariables = collection.variableIds.length > 0;
    const hasModes = collection.modes.length > 1;

    if (useThemeExtensions && hasVariables && hasModes) {
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

      dartCode += `List<Map<String, dynamic>> get map;`;
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
        dartCode += `  static IApp${className} get ${removeSpacesAndDigits(
          mode.name.toLowerCase()
        )}${className} => const App${className}(\n`;

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

      // Генерация props для класса
      dartCode += `  @override\n  List<Map<String, dynamic>> get map => [\n`;
      for (const mode of collection.modes) {
        for (const variableId of collection.variableIds) {
          const variable = await figma.variables.getVariableByIdAsync(
            variableId
          );

          const variableName = formatVariableName(variable?.name ?? "");

          dartCode += `    {'${variableName}' : ${variableName}},\n`;
        }
      }
      dartCode += "  ];\n";

      dartCode += "}\n";
    } else {
      dartCode += `final class App${formatClassName(
        collection.name
      )} {\n  const App${formatClassName(collection.name)}._();\n\n`;

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
              dartCode += `  static const Color ${formatVariableNameWMode(
                mode.name,
                hasModes,
                variable?.name ?? "null"
              )} = ${colorCode};\n`;
            } else if (aliasVariable?.resolvedType === "FLOAT") {
              dartCode += `  static const double ${formatVariableNameWMode(
                mode.name,
                hasModes,
                variable?.name ?? "null"
              )} = ${aliasValue};\n`;
            } else {
              dartCode += `  static const dynamic ${formatVariableNameWMode(
                mode.name,
                hasModes,
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
            dartCode += `  /// ${variable?.name} = ${colorCode}\n`;
            dartCode += `  static const Color ${formatVariableNameWMode(
              mode.name,
              hasModes,
              variable?.name ?? "null"
            )} = ${colorCode};\n`;
          } else if (variable?.resolvedType === "FLOAT") {
            dartCode += `  /// ${variable?.name} = ${value}\n`;
            dartCode += `  static const double ${formatVariableNameWMode(
              mode.name,
              hasModes,
              variable?.name ?? "null"
            )} = ${value};\n`;
          } else {
            dartCode += `  /// ${variable?.name} = ${value}\n`;
            dartCode += `  static const dynamic ${formatVariableNameWMode(
              mode.name,
              hasModes,
              variable?.name ?? "null"
            )} = ${value};\n`;
          }
        }
      }

      // Генерация props для класса
      dartCode += `\n  static List<Map<String, dynamic>> get map => [\n`;
      for (const mode of collection.modes) {
        for (const variableId of collection.variableIds) {
          const variable = await figma.variables.getVariableByIdAsync(
            variableId
          );

          const variableName = formatVariableName(variable?.name ?? "");

          dartCode += `    {'${variableName}':${variableName}},\n`;
        }
      }
      dartCode += "  ];\n";

      dartCode += `}\n\n`;
    }
  }

  return dartCode;
}

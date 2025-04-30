// Types for better code clarity
type VariableType = "COLOR" | "FLOAT" | "STRING" | "BOOLEAN";
type TypeMapping = {
  COLOR: "Color";
  FLOAT: "double";
  STRING: "String";
  BOOLEAN: "bool";
};

interface VariableData {
  variable: Variable | null;
  name: string;
  variableType: string;
  dartType: string;
}

function getColorCode(color: RGBA): string {
  return `Color(0x${toHex(color.a)}${toHex(color.r)}${toHex(color.g)}${toHex(
    color.b
  )})`;
}

async function processVariable(variableId: string): Promise<VariableData> {
  const variable = await figma.variables.getVariableByIdAsync(variableId);
  const name = formatVariableName(variable?.name ?? "");

  let variableType = "dynamic";
  let dartType = "dynamic";

  if (variable?.resolvedType === "COLOR") {
    variableType = "Color";
    dartType = "Color";
  } else if (variable?.resolvedType === "FLOAT") {
    variableType = "double";
    dartType = "double";
  }

  return { variable, name, variableType, dartType };
}

async function resolveAliasValue(
  value: VariableAlias
): Promise<{ value: any; type: VariableType }> {
  const aliasVariable = await figma.variables.getVariableByIdAsync(value.id);
  const aliasValue = aliasVariable?.valuesByMode["1:0"];
  return {
    value: aliasValue,
    type: aliasVariable?.resolvedType as VariableType,
  };
}

async function formatValue(value: any, type: VariableType): Promise<string> {
  if (
    type === "COLOR" &&
    typeof value === "object" &&
    value !== null &&
    "r" in value
  ) {
    const asColor = "a" in value ? (value as RGBA) : { ...value, a: 1 };
    return getColorCode(asColor);
  }
  return `${value}`;
}

// Main class for generating variable code
class DartVariableGenerator {
  private dartCode: string = "";

  constructor() {
    this.dartCode = initialize();
  }

  async generateThemeExtensionInterface(
    className: string,
    variableIds: string[]
  ): Promise<void> {
    this.dartCode += `abstract interface class IApp${className} extends ThemeExtension<IApp${className}> {\n`;

    // Abstract properties
    for (const variableId of variableIds) {
      const { name, dartType } = await processVariable(variableId);
      this.dartCode += `  abstract final ${dartType} ${name};\n`;
    }

    this.dartCode += `  List<Map<String, dynamic>> get map;\n`;
    this.dartCode += "}\n\n";
  }

  async generateThemeExtensionImplementation(
    className: string,
    collection: VariableCollection,
    variableIds: string[]
  ): Promise<void> {
    this.dartCode += `@immutable\nfinal class App${className} implements IApp${className} {\n`;

    // Constructor
    this.dartCode += `  const App${className}({\n`;
    for (const variableId of variableIds) {
      const { variable, name } = await processVariable(variableId);
      if (!variable) continue;
      this.dartCode += `    required this.${name},\n`;
    }
    this.dartCode += "  });\n\n";

    // Properties
    for (const variableId of variableIds) {
      const { variable, name, dartType } = await processVariable(variableId);
      if (!variable) continue;
      this.dartCode += `  @override\n  final ${dartType} ${name};\n`;
    }

    await this.generateCopyWithMethod(className, variableIds);
    await this.generateLerpMethod(className, variableIds);
    await this.generateEqualsMethod(className, variableIds);
    await this.generateHashCodeMethod(variableIds);

    // Type getter
    this.dartCode += `  @override\n  Object get type => runtimeType;\n\n`;

    // Mode-specific instances
    await this.generateModeInstances(className, collection, variableIds);

    // Map getter
    await this.generateMapGetter(collection, variableIds);

    this.dartCode += "}\n";
  }

  async generateCopyWithMethod(
    className: string,
    variableIds: string[]
  ): Promise<void> {
    this.dartCode += `\n  @override\n  App${className} copyWith({\n`;

    for (const variableId of variableIds) {
      const { variable, name, dartType } = await processVariable(variableId);
      if (!variable) continue;
      this.dartCode += `    ${dartType}? ${name},\n`;
    }

    this.dartCode += `  }) => App${className}(\n`;
    for (const variableId of variableIds) {
      const { variable, name } = await processVariable(variableId);
      if (!variable) continue;
      this.dartCode += `    ${name}: ${name} ?? this.${name},\n`;
    }
    this.dartCode += "  );\n\n";
  }

  async generateLerpMethod(
    className: string,
    variableIds: string[]
  ): Promise<void> {
    this.dartCode += `  @override\n  App${className} lerp(App${className}? other, double t) {\n`;
    this.dartCode += "    if (other == null) return this;\n";
    this.dartCode += `    return App${className}(\n`;

    for (const variableId of variableIds) {
      const { variable, name, variableType } = await processVariable(
        variableId
      );
      if (!variable) continue;

      if (variableType === "Color") {
        this.dartCode += `      ${name}: Color.lerp(${name}, other.${name}, t)!,\n`;
      } else if (variableType === "double") {
        this.dartCode += `      ${name}: lerpDouble(${name}, other.${name}, t)!,\n`;
      } else {
        this.dartCode += `      ${name}: other.${name} ?? this.${name},\n`;
      }
    }
    this.dartCode += "    );\n  }\n";
  }

  async generateEqualsMethod(
    className: string,
    variableIds: string[]
  ): Promise<void> {
    this.dartCode += `\n  @override\n  bool operator ==(Object other) {\n`;
    this.dartCode += `    if (identical(this, other)) return true;\n`;
    this.dartCode += `    return other is App${className} &&\n`;

    let equalsConditions = [];
    for (const variableId of variableIds) {
      const { variable, name } = await processVariable(variableId);
      if (!variable) continue;
      equalsConditions.push(`      other.${name} == ${name}`);
    }

    this.dartCode += equalsConditions.join(" &&\n") + ";\n  }\n";
  }

  async generateHashCodeMethod(variableIds: string[]): Promise<void> {
    this.dartCode += `\n  @override\n  int get hashCode =>\n`;
    this.dartCode += "    Object.hashAll([\n";

    for (const variableId of variableIds) {
      const { variable, name } = await processVariable(variableId);
      if (!variable) continue;
      this.dartCode += `      ${name},\n`;
    }

    this.dartCode += "    ]);\n\n";
  }

  async generateModeInstances(
    className: string,
    collection: VariableCollection,
    variableIds: string[]
  ): Promise<void> {
    for (const mode of collection.modes) {
      const modeName = removeSpacesAndDigits(mode.name.toLowerCase());
      this.dartCode += `  static IApp${className} get ${modeName}${className} => const App${className}(\n`;

      for (const variableId of variableIds) {
        const { variable, name } = await processVariable(variableId);
        if (!variable) continue;

        const value = variable.valuesByMode[mode.modeId];

        if (typeof value === "object" && value !== null && "type" in value) {
          // Handle alias variables
          const { value: aliasValue, type: aliasType } =
            await resolveAliasValue(value as VariableAlias);
          const formattedValue = await formatValue(aliasValue, aliasType);
          this.dartCode += `    ${name}: ${formattedValue},\n`;
        } else if (
          typeof value === "object" &&
          value !== null &&
          "r" in value
        ) {
          // Handle color values
          const asColor = value as RGBA;
          const colorCode = getColorCode(asColor);
          this.dartCode += `    ${name}: ${colorCode},\n`;
        } else {
          // Handle other values
          this.dartCode += `    ${name}: ${value},\n`;
        }
      }

      this.dartCode += ");\n\n";
    }
  }

  async generateStandardClass(
    className: string,
    collection: VariableCollection
  ): Promise<void> {
    this.dartCode += `final class App${className} {\n`;
    this.dartCode += `  const App${className}._();\n\n`;

    const hasModes = collection.modes.length > 1;

    for (const mode of collection.modes) {
      for (const variableId of collection.variableIds) {
        const variable = await figma.variables.getVariableByIdAsync(variableId);
        if (!variable) continue;

        const value = variable.valuesByMode[mode.modeId];
        const formattedName = formatVariableNameWMode(
          mode.name,
          hasModes,
          variable.name
        );

        // Add comment for the variable
        this.dartCode += `  /// ${variable.name}\n`;

        if (typeof value === "object" && value !== null && "type" in value) {
          // Handle alias variables
          const { value: aliasValue, type: aliasType } =
            await resolveAliasValue(value as VariableAlias);
          const formattedValue = await formatValue(aliasValue, aliasType);
          const dartType =
            aliasType === "COLOR"
              ? "Color"
              : aliasType === "FLOAT"
              ? "double"
              : "dynamic";

          this.dartCode += `  static const ${dartType} ${formattedName} = ${formattedValue};\n`;
        } else if (
          typeof value === "object" &&
          value !== null &&
          "r" in value
        ) {
          // Handle color values
          const asColor = "a" in value ? (value as RGBA) : { ...value, a: 1 };
          const colorCode = getColorCode(asColor);
          this.dartCode += `  static const Color ${formattedName} = ${colorCode};\n`;
        } else if (variable.resolvedType === "FLOAT") {
          this.dartCode += `  static const double ${formattedName} = ${value};\n`;
        } else {
          this.dartCode += `  static const dynamic ${formattedName} = ${value};\n`;
        }
      }
    }

    // Generate map getter
    await this.generateMapGetter(collection, collection.variableIds);

    this.dartCode += "}\n\n";
  }

  async generateMapGetter(
    collection: VariableCollection,
    variableIds: string[]
  ): Promise<void> {
    this.dartCode += `  @override\n  List<Map<String, dynamic>> get map => [\n`;

    for (const variableId of variableIds) {
      const { name } = await processVariable(variableId);
      this.dartCode += `    {'${name}': ${name}},\n`;
    }

    this.dartCode += "  ];\n";
  }

  getDartCode(): string {
    return this.dartCode;
  }
}

// Main function
async function generateVariables(useThemeExtensions: boolean): Promise<string> {
  const collections = await figma.variables.getLocalVariableCollectionsAsync();
  const generator = new DartVariableGenerator();

  for (const collection of collections) {
    const className = formatClassName(collection.name);
    const hasVariables = collection.variableIds.length > 0;
    const hasModes = collection.modes.length > 1;

    if (useThemeExtensions && hasVariables && hasModes) {
      await generator.generateThemeExtensionInterface(
        className,
        collection.variableIds
      );
      await generator.generateThemeExtensionImplementation(
        className,
        collection,
        collection.variableIds
      );
    } else {
      await generator.generateStandardClass(className, collection);
    }
  }

  return generator.getDartCode();
}

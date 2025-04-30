async function generateTextStyles(
  useThemeExtensions: boolean,
  includeFontName: boolean
): Promise<string> {
  try {
    const textStyles = await figma.getLocalTextStylesAsync();
    if (textStyles.length === 0) {
      return "No defined textstyles";
    }

    let dartCode = initialize();

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
          dartCode += `          fontFamily: '${fontFamily}',\n`;
        }

        if (lineHeightValue !== "null") {
          const height = Math.round((lineHeightValue / fontSize) * 100) / 100;
          dartCode += `          height: ${height},\n`;
        }

        if (letterSpacing != null && letterSpacing !== 0) {
          console.log("letterSpacing", letterSpacing, fontSize);
          //  const roundedLetterSpacing = Number(letterSpacing.toFixed(1));
          dartCode += `          letterSpacing: ${letterSpacing},\n`;
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
      // Генерация props для класса
      dartCode += `  \n  static List<Map<String, dynamic>> get map => [\n`;
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

        dartCode += `    {'${formattedStyleName}': ${formattedStyleName}},\n`;
      });
      dartCode += "  ];\n";
    }

    dartCode += "}\n";
    return dartCode;
  } catch (error) {
    console.error("An error occurred:", error);
    return "";
  }
}

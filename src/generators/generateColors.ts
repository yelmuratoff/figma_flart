async function generateColors(): Promise<string> {
  try {
    const localColorStyles = await figma.getLocalPaintStylesAsync();

    if (localColorStyles.length === 0) {
      return "No defined colors";
    }

    let dartCode = initialize();

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

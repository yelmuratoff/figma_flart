async function generateEffectStyles(): Promise<string> {
  try {
    const localEffectStyles = await figma.getLocalEffectStylesAsync();

    if (localEffectStyles.length === 0) {
      return "No defined effect styles";
    }

    let dartCode = initialize();

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

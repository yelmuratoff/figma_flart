import { generateTextStyles, collectTextStyleModes } from "./generators/generateTextStyles";
import { generateColors, collectColorModes } from "./generators/generateColors";
import { generateEffectStyles, collectEffectModes } from "./generators/generateEffectStyles";
import { generateVariables, collectVariableModes } from "./generators/generateVariables";

if (figma.editorType === "figma" || figma.editorType === "dev") {
  figma.showUI(__html__, { width: 650, height: 850 });

  figma.ui.onmessage = async (msg) => {
    // ── TextStyles ──
    if (msg.type === "generate-textstyles") {
      const modes = await collectTextStyleModes();
      if (modes && modes.length > 1) {
        figma.ui.postMessage({
          type: "select-mode",
          generator: "textstyles",
          modes,
          useThemeExtensions: msg.useThemeExtensions,
          includeFontName: msg.includeFontName,
        });
      } else {
        const dartCode = await generateTextStyles(
          msg.useThemeExtensions,
          msg.includeFontName
        );
        figma.ui.postMessage({ type: "dart-code", code: dartCode });
      }
    }

    // ── Colors ──
    if (msg.type === "generate-colors") {
      const modes = await collectColorModes();
      if (modes && modes.length > 1) {
        figma.ui.postMessage({
          type: "select-mode",
          generator: "colors",
          modes,
        });
      } else {
        const dartCode = await generateColors();
        figma.ui.postMessage({ type: "dart-code", code: dartCode });
      }
    }

    // ── Effects ──
    if (msg.type === "generate-effects") {
      const modes = await collectEffectModes();
      if (modes && modes.length > 1) {
        figma.ui.postMessage({
          type: "select-mode",
          generator: "effects",
          modes,
        });
      } else {
        const dartCode = await generateEffectStyles();
        figma.ui.postMessage({ type: "dart-code", code: dartCode });
      }
    }

    // ── Variables ──
    if (msg.type === "generate-variables") {
      const modes = await collectVariableModes();
      if (modes && modes.length > 1) {
        figma.ui.postMessage({
          type: "select-mode",
          generator: "variables",
          modes,
          useThemeExtensions: msg.useThemeExtensions,
        });
      } else {
        const dartCode = await generateVariables(msg.useThemeExtensions);
        figma.ui.postMessage({ type: "dart-code", code: dartCode });
      }
    }

    // ── Generate with selected modes ──
    if (msg.type === "generate-with-mode") {
      let dartCode = "";
      const modeIds: string[] = msg.modeIds;

      if (msg.generator === "textstyles") {
        dartCode = await generateTextStyles(
          msg.useThemeExtensions,
          msg.includeFontName,
          modeIds
        );
      } else if (msg.generator === "colors") {
        dartCode = await generateColors(modeIds);
      } else if (msg.generator === "effects") {
        dartCode = await generateEffectStyles(modeIds);
      } else if (msg.generator === "variables") {
        dartCode = await generateVariables(msg.useThemeExtensions, modeIds);
      }

      figma.ui.postMessage({ type: "dart-code", code: dartCode });
    }
  };
}

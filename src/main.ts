import { generateTextStyles, collectTextStyleModes } from "./generators/generateTextStyles";
import { generateColors } from "./generators/generateColors";
import { generateEffectStyles } from "./generators/generateEffectStyles";
import { generateVariables } from "./generators/generateVariables";

if (figma.editorType === "figma" || figma.editorType === "dev") {
  figma.showUI(__html__, { width: 650, height: 850 });

  figma.ui.onmessage = async (msg) => {
    if (msg.type === "generate-textstyles") {
      const useThemeExtensions = msg.useThemeExtensions;
      const includeFontName = msg.includeFontName;

      // Check if text styles have variable modes
      const modes = await collectTextStyleModes();
      if (modes && modes.length > 1) {
        // Send modes to UI for user selection
        figma.ui.postMessage({
          type: "select-mode",
          modes: modes,
          useThemeExtensions,
          includeFontName,
        });
      } else {
        // No modes — generate directly
        let dartCode = await generateTextStyles(
          useThemeExtensions,
          includeFontName
        );
        figma.ui.postMessage({ type: "dart-code", code: dartCode });
      }
    }

    if (msg.type === "generate-textstyles-with-mode") {
      const dartCode = await generateTextStyles(
        msg.useThemeExtensions,
        msg.includeFontName,
        msg.modeId
      );
      figma.ui.postMessage({ type: "dart-code", code: dartCode });
    }

    if (msg.type === "generate-colors") {
      let dartCode = await generateColors();
      figma.ui.postMessage({ type: "dart-code", code: dartCode });
    }

    if (msg.type === "generate-effects") {
      let dartCode = await generateEffectStyles();
      figma.ui.postMessage({ type: "dart-code", code: dartCode });
    }

    if (msg.type === "generate-variables") {
      const useThemeExtensions = msg.useThemeExtensions;
      let dartCode = await generateVariables(useThemeExtensions);
      figma.ui.postMessage({ type: "dart-code", code: dartCode });
    }
  };
}

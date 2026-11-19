// Serves an assembled flart snapshot through the six Plugin API calls the
// 1.1.1 generators make, so they run in Node unchanged.

type Snapshot = {
  collections: any[];
  variables: any[];
  textStyles: any[];
  paintStyles: any[];
  effectStyles: any[];
};

export function installFigma(snapshot: Snapshot): void {
  const variables = new Map(snapshot.variables.map((v) => [v.id, v]));
  const collections = new Map(snapshot.collections.map((c) => [c.id, c]));
  (globalThis as any).figma = {
    getLocalTextStylesAsync: async () => snapshot.textStyles,
    getLocalPaintStylesAsync: async () => snapshot.paintStyles,
    getLocalEffectStylesAsync: async () => snapshot.effectStyles,
    variables: {
      getLocalVariableCollectionsAsync: async () =>
        snapshot.collections.filter((c) => !c.remote),
      getVariableByIdAsync: async (id: string) => variables.get(id) ?? null,
      getVariableCollectionByIdAsync: async (id: string) =>
        collections.get(id) ?? null,
    },
  };
}

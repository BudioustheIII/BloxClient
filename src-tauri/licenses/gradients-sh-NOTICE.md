# gradients.sh

The gradient surface and gradient engine in this folder come from the free gradients.sh
registry and are MIT licensed: https://gradients.sh/license

- Source: `https://gradients.sh/r/gradient-surface.json` and `gradient-engine.json`,
  downloaded 2026-11-23.
- `mercury.tsx` applies the settings of the free Mercury preset:
  https://gradients.sh/studio/mercury
- Local changes: registry import paths were rewritten to relative paths, `cn.ts` replaces the
  shadcn utility, and the engine gained a frame-rate cap, GPU backpressure and startup retries.
- The engine's simplex noise is credited to Ian McEwan, Stefan Gustavson and Ashima Arts (MIT).

Keep this notice and any copyright notices in these files. The registry files did not name a
copyright holder at download time.

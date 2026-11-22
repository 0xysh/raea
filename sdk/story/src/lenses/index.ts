// The lenses raea ships, by name (docs/story.feature ST3a, ST3b).
import type { Lens } from "../lens.ts";
import { mckee } from "./mckee.ts";
import { pas } from "./pas.ts";
import { pixar } from "./pixar.ts";
import { storynomics } from "./storynomics.ts";

export { mckee, pas, pixar, storynomics };
export const LENSES: Record<string, Lens> = { mckee, pixar, storynomics, pas };

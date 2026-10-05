// A club's kit colours for the line-up sheets and the kick-off dots: the shirt colour made
// safe, with the number in whichever of paper and ink reads on it. The same rule as the crest
// and the pitch markers, so each club is drawn in one colour on every screen.

import { SKIN_TOKENS, TOKENS, safeKit } from './colour.js';

export function teamKit(team) {
  if (!team) {
    return null;
  }
  const theme = globalThis.document?.documentElement.dataset.theme;
  return safeKit(
    { primary: team['team.kit.primary'], secondary: team['team.kit.secondary'] },
    SKIN_TOKENS[theme] ?? TOKENS
  );
}

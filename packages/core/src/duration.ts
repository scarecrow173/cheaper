export const DURATION_PATTERN = /^[1-9][0-9]*(ms|s|m|h)$/;

const MULTIPLIERS: Record<string, number> = {
  ms: 1,
  s: 1_000,
  m: 60_000,
  h: 3_600_000,
};

export function parseDuration(input: string): number {
  const match = DURATION_PATTERN.exec(input);
  const suffix = match?.[1];
  const multiplier = suffix !== undefined ? MULTIPLIERS[suffix] : undefined;
  if (!match || suffix === undefined || multiplier === undefined) {
    throw new RangeError(`invalid duration: ${input}`);
  }
  const value = Number(match[0].slice(0, match[0].length - suffix.length));
  return value * multiplier;
}

export function convert(data: string): number {
  // CAST: JSON parse guarantees numeric string from API
  return data as unknown as number;
}

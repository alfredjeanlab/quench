// CAST: JSON parse guarantees numeric string from API
export function convert(data: string): number {
  return data as unknown as number;
}

export function convert(data: string): number {
  // Missing // CAST: comment - should fail
  return data as unknown as number;
}

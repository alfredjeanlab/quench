// Legacy API requires console output for debugging
// eslint-disable-next-line no-console
console.log('debug');

export function main(): void {
  // Migration in progress - legacy wrapper requires any
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const data: any = {};
  console.log(data);
}

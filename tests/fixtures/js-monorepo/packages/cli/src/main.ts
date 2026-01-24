/**
 * CLI entry point for js-monorepo fixture.
 */

import { createConfig, formatConfig, validateConfig } from '@js-monorepo/core';

/**
 * Parse command line arguments.
 */
export function parseArgs(args: string[]): { name: string; debug: boolean } {
  const name = args[0] ?? 'default';
  const debug = args.includes('--debug');
  return { name, debug };
}

/**
 * Run the CLI application.
 */
export function run(args: string[]): number {
  const { name, debug } = parseArgs(args);
  const config = createConfig(name);
  config.debug = debug;

  if (!validateConfig(config)) {
    console.error('Invalid configuration');
    return 1;
  }

  console.log(`Running: ${formatConfig(config)}`);
  return 0;
}

// Main entry point
if (typeof process !== 'undefined') {
  const exitCode = run(process.argv.slice(2));
  process.exit(exitCode);
}

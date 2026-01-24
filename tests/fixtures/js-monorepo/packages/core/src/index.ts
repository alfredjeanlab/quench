/**
 * Core library for js-monorepo fixture.
 */

/**
 * Configuration options for the application.
 */
export interface Config {
  name: string;
  version: string;
  debug?: boolean;
}

/**
 * Create a default configuration.
 */
export function createConfig(name: string): Config {
  return {
    name,
    version: '1.0.0',
    debug: false,
  };
}

/**
 * Validate a configuration object.
 */
export function validateConfig(config: Config): boolean {
  return config.name.length > 0 && config.version.length > 0;
}

/**
 * Format configuration as a string.
 */
export function formatConfig(config: Config): string {
  return `${config.name}@${config.version}`;
}

/**
 * Tests for core library.
 */

import { createConfig, validateConfig, formatConfig, Config } from '../src/index';

describe('createConfig', () => {
  it('creates config with name', () => {
    const config = createConfig('test-app');
    expect(config.name).toBe('test-app');
    expect(config.version).toBe('1.0.0');
    expect(config.debug).toBe(false);
  });
});

describe('validateConfig', () => {
  it('validates valid config', () => {
    const config: Config = { name: 'app', version: '1.0.0' };
    expect(validateConfig(config)).toBe(true);
  });

  it('rejects empty name', () => {
    const config: Config = { name: '', version: '1.0.0' };
    expect(validateConfig(config)).toBe(false);
  });
});

describe('formatConfig', () => {
  it('formats config as string', () => {
    const config: Config = { name: 'myapp', version: '2.0.0' };
    expect(formatConfig(config)).toBe('myapp@2.0.0');
  });
});

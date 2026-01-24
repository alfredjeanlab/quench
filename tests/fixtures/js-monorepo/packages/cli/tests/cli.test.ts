/**
 * Tests for CLI.
 */

import { parseArgs, run } from '../src/main';

describe('parseArgs', () => {
  it('parses name from first arg', () => {
    const result = parseArgs(['myapp']);
    expect(result.name).toBe('myapp');
    expect(result.debug).toBe(false);
  });

  it('defaults name when no args', () => {
    const result = parseArgs([]);
    expect(result.name).toBe('default');
  });

  it('parses debug flag', () => {
    const result = parseArgs(['app', '--debug']);
    expect(result.debug).toBe(true);
  });
});

describe('run', () => {
  it('returns 0 on success', () => {
    const code = run(['test-app']);
    expect(code).toBe(0);
  });

  it('returns 0 with debug', () => {
    const code = run(['app', '--debug']);
    expect(code).toBe(0);
  });
});

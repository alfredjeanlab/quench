/**
 * Tests for index.ts
 */

import { greet, calculate } from '../src/index';

describe('greet', () => {
  it('returns greeting with name', () => {
    expect(greet('World')).toBe('Hello, World!');
  });

  it('handles empty name', () => {
    expect(greet('')).toBe('Hello, !');
  });
});

describe('calculate', () => {
  it('returns sum and product', () => {
    const result = calculate(3, 4);
    expect(result.sum).toBe(7);
    expect(result.product).toBe(12);
  });

  it('handles zeros', () => {
    const result = calculate(0, 5);
    expect(result.sum).toBe(5);
    expect(result.product).toBe(0);
  });
});

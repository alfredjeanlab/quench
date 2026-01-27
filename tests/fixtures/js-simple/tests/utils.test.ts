/**
 * Tests for utils.ts
 */

import { describe, it, expect } from 'vitest';
import { add, multiply, isPositive } from '../src/utils';

describe('add', () => {
  it('adds positive numbers', () => {
    expect(add(2, 3)).toBe(5);
  });

  it('adds negative numbers', () => {
    expect(add(-2, -3)).toBe(-5);
  });
});

describe('multiply', () => {
  it('multiplies positive numbers', () => {
    expect(multiply(2, 3)).toBe(6);
  });

  it('multiplies with zero', () => {
    expect(multiply(5, 0)).toBe(0);
  });
});

describe('isPositive', () => {
  it('returns true for positive', () => {
    expect(isPositive(5)).toBe(true);
  });

  it('returns false for zero', () => {
    expect(isPositive(0)).toBe(false);
  });

  it('returns false for negative', () => {
    expect(isPositive(-1)).toBe(false);
  });
});

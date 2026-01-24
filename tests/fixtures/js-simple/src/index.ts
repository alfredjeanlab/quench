/**
 * Main entry point for js-simple fixture.
 */

import { add, multiply } from './utils';

/**
 * Greet a user by name.
 */
export function greet(name: string): string {
  return `Hello, ${name}!`;
}

/**
 * Calculate the sum and product of two numbers.
 */
export function calculate(a: number, b: number): { sum: number; product: number } {
  return {
    sum: add(a, b),
    product: multiply(a, b),
  };
}

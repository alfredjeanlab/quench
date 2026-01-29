import { covered } from '../src/math';
import { test, expect } from 'vitest';
test('covered function', () => { expect(covered()).toBe(42); });

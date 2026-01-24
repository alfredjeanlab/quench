export function setupTest() {
  return { ready: true };
}

test('setup works', () => {
  expect(setupTest().ready).toBe(true);
});

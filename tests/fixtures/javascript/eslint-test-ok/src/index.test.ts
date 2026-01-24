describe('test', () => {
  it('allows eslint-disable in test file without comment', () => {
    // eslint-disable-next-line no-console
    console.log('debug in test');
    expect(true).toBe(true);
  });
});

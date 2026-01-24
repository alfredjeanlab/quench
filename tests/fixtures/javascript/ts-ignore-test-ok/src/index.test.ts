describe('test', () => {
  it('allows ts-ignore in test file', () => {
    // @ts-ignore - testing error case
    const invalid: number = 'string';
    expect(typeof invalid).toBe('string');
  });
});

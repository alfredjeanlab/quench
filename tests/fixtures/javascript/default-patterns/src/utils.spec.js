const { helper } = require('./utils');

describe('helper', () => {
  it('returns a number', () => {
    expect(typeof helper()).toBe('number');
  });
});

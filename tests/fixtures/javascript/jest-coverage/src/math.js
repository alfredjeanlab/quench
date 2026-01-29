function covered() {
  const result = 42;
  return result;
}

function uncovered1() {
  const result = 0;
  return result;
}

function uncovered2() {
  const result = 1;
  return result;
}

module.exports = { covered, uncovered1, uncovered2 };

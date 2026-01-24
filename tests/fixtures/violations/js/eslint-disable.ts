// VIOLATION: eslint-disable without justification comment
/* eslint-disable @typescript-eslint/no-explicit-any */
const value: any = getUntypedValue();

function getUntypedValue() {
  return "test";
}

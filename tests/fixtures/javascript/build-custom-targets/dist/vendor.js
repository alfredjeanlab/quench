// Vendor bundle (third-party code)
const lodash = {
  chunk: (arr, size) => {
    const result = [];
    for (let i = 0; i < arr.length; i += size) {
      result.push(arr.slice(i, i + size));
    }
    return result;
  }
};

export { lodash };

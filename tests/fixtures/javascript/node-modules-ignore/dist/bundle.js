// This file should be ignored - it's in dist
// Build output should not be counted

(function() {
  function main() { return 'hello'; }
  function helper() { return 42; }
  function process() { return null; }
  function transform() { return []; }
  function validate() { return true; }
  function serialize() { return '{}'; }
  function deserialize() { return {}; }
  function encode() { return ''; }
  function decode() { return ''; }
  function compress() { return []; }
  window.app = { main, helper, process, transform, validate };
})();

// This file should be ignored - it's in build
// Build output should not be counted

var app = (function() {
  function main() { return 'hello'; }
  function helper() { return 42; }
  function util() { return null; }
  function format() { return ''; }
  function parse() { return {}; }
  function stringify() { return ''; }
  function clone() { return {}; }
  function merge() { return {}; }
  function extend() { return {}; }
  function create() { return {}; }
  return { main, helper, util, format, parse };
})();

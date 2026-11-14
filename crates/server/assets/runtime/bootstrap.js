/* dshr shared host runtime (tier B PoC): the module table every opted-in
 * client-half frame gets before its bundle. React/ReactDOM UMD builds are
 * concatenated above this file by the server route; this glue publishes the
 * reference module ids in window.__DSHR_MODULES__, including a jsx-runtime shim
 * built on React.createElement (the UMD build ships no jsx-runtime module).
 * A plugin-shipped dsh.client.runtime runs afterwards and may override entries. */
(function () {
  var React = window.React;
  var ReactDOM = window.ReactDOM;
  if (!React || !ReactDOM) {
    throw new Error("dshr shared runtime: React UMD did not publish window.React/window.ReactDOM");
  }
  function jsx(type, props, key) {
    var next = {};
    if (props) {
      for (var name in props) next[name] = props[name];
    }
    if (key !== undefined) next.key = key;
    return React.createElement(type, next);
  }
  var jsxRuntime = { jsx: jsx, jsxs: jsx, Fragment: React.Fragment };
  window.__DSHR_MODULES__ = Object.assign(window.__DSHR_MODULES__ || {}, {
    "react": React,
    "react/jsx-runtime": jsxRuntime,
    "react/jsx-dev-runtime": { jsx: jsx, jsxs: jsx, jsxDEV: jsx, Fragment: React.Fragment },
    "react-dom": ReactDOM,
    "react-dom/client": ReactDOM,
  });
})();

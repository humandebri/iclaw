// where: standalone/web/src/test-setup.ts
// what: Shared vitest DOM test setup
// why: React 19 component tests should run in a declared act environment to keep test output clean

Object.defineProperty(globalThis, "IS_REACT_ACT_ENVIRONMENT", {
  value: true,
  configurable: true,
});

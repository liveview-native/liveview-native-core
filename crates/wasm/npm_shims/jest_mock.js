/// Substitute live view webs classes for our own during jest tests.
jest.mock("phoenix_live_view/Rendered", () => {
  const wasmProxy = require("liveview_native_core_wasm");
  return wasmProxy.Rendered;
});

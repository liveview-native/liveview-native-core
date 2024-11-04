/// Substitute live view webs classes for our own during jest tests.
jest.mock("phoenix_live_view/rendered", () => {
  const wasmProxy = require("liveview_native_core_wasm_nodejs");
  return wasmProxy.Rendered;
});

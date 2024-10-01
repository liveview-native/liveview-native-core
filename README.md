# LiveView Native Core

This repository contains an implementation of the LiveView Native core library, which is intended to handle all
the details which are common across the various platforms on which LiveView Native is used. 

## Features

Currently, the features planned to be provided by the Core library consists of:

1. Connection management for the LiveView socket
2. Initializing a virtual DOM from the page received from the LiveView server, and associated parsing out of components, live views, and statics
3. Receipt, decoding, and application of changes sent by the LiveView server to the virtual DOM managed by Core
4. Management of callbacks/event handlers registered from the host language in response to virtual DOM changes

Host languages (e.g. Swift) can then bind to this library, and focus on the details of rendering the UI, rather than on how to interact with the
LiveView server.

## Status

This library is not quite ready for production use yet, as there are still some details of the API being finalized, and much testing remains to
be done. We'll plan to announce the library more publicly when it is ready for the community to build with.

## License

Apache 2.0. See `LICENSE.md`.

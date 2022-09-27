# LiveView Core

## Terminology

- "LV", Phoenix LiveView
- "LVN", LiveView Native
- "host platform", the native UI framework and associated code for rendering via that framework (e.g. SwiftUI)
- "host bindings", the layer that sits between the host platform and Core and handles the details of initializing Core and calling Core APIs
- "server", the Phoenix LiveView server/backend
- "client", the LVN client, i.e. Core + host bindings
- "DOM", the abstract element tree maintained by Core, derived from a server-provided template + LV updates
- "view tree", the native UI view/element tree, derived from the DOM

## Architecture

Core will consist of two main user-facing components:

- A "DOM", an abstract tree of nodes, each of which has one or more attributes. These can be serialized/deserialized from strings (in either XML or JSON form), diffed, merged, traversed and either destructively or non-destructively modified. Many of these may exist in memory at the same time.
- A "client", which is instantiated per-view, and holds a DOM internally to which it applies updates as they are received. Callbacks may be registered with the client to be notified on various events; such as when the DOM is loaded, when it is updated, and more.

To use Core, a set of native bindings must be implemented for each host platform that expose the functionality provided by Core in a way that is best suited for the host language.

## Usage

### Version 1

In the near-term, we are punting on a few things to allow integrating with our existing SwiftUI client as quickly as possible.
As a result, while it has some deficiencies, this will allow us to validate Core as soon as possible and ultimately iterate more quickly.

In this version, Core is used like so:

1. Host creates a Core client, providing the client with whatever configuration it needs, including registering any callbacks for events the Host cares about.
To be useful though, the Host must register a callback which will be invoked when the DOM is updated, which it can then use to (re)render the native UI view tree
2. Host establishes a connection to an LV server, receiving the initial server-rendered template as a string
3. Host calls an API on the client that tells it to initialize its internal DOM from a given template string, setting aside that template for its own use if desired
4. As the LV server continues to periodically send updates over the wire, to be applied against the template it originally sent,
the Host will parse these updates as it sees fit, and when ready to apply them, will generate a new template representing the updated document.
5. Host calls a Core API to create a fresh DOM from the updated template it generated
6. Host calls an API on the client with the new DOM that tells it to perform a diff+merge of its internal DOM against the given DOM
7. The Core client will invoke its registered callbacks any time its internal DOM changes, causing the Host to re-render its view tree

There are a few awkward bits here:

- The responsibility for communicating with the LV server should really be owned by the Core client, but in this version is owned by the Host
- The Host must perform redundant serialization/deserialization of LV updates multiple times
- The Host must maintain additional state so that it can generate templates for the Core client to parse and apply as diffs to its DOM

So, in a future version, we plan to address these deficiencies as described below.

### Version 2

There is a lot of overlap between this version and the previous, but it is considerably simpler and resolves all of the issues raised above:

1. Host creates a Core client, as described in Version 1, but the configuration is extended to include things necessary for connecting to the LV server
2. Core client establishes a connection to the LV server, receives the initial server-rendered template, and initializes its internal DOM using it
3. Core client invokes the registered callback to tell the Host that it should render now
4. LV server continues to periodically send updates over the wire, which the Core client receives, decodes, and applies to its DOM. Each time a batch of 
these updates are applied, the Host-provided callback is invoked again with a handle to the latest DOM

Since the client owns the LV server connection in this version, we would want to expose the ability to register callbacks for additional events, such as
server disconnects/reconnects, errors decoding the DOM/updates, and anything else deemed worth propagating to the host.

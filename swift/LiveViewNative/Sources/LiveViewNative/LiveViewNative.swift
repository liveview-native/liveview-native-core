import LiveViewNativeCore

/// Raised when a `Document` fails to parse
public struct ParseError: Error {
    let message: String

    init(message: String) {
        self.message = message
    }
}

/// Represents the various types of events that a `Document` can produce
public enum EventType {
    /// When a document is modified in some way, the `changed` event is raised
    case changed
}

struct Handler {
    let context: AnyObject?
    let callback: (Document, AnyObject?) -> ()

    func call(_ doc: Document) {
        callback(doc, context)
    }
}

/// A `Document` corresponds to the tree of elements in a UI, and supports a variety
/// of operations used to traverse, query, and mutate that tree.
public class Document {
    var repr: __Document
    var handlers: [EventType: Handler] = [:]

    init(_ doc: __Document) {
        self.repr = doc
    }

    deinit {
        __liveview_native_core$Document$drop(self.repr)
    }

    /// Parse a `Document` from the given `String` or `String`-like type
    ///
    /// The given text should be a valid HTML-ish document, insofar that the structure should
    /// be that of an HTML document, but the tags, attributes, and their usages do not have to
    /// be valid according to the HTML spec.
    ///
    /// - Parameters:
    ///   - str: The string to parse
    ///
    /// - Returns: A document representing the parsed content
    ///
    /// - Throws: `ParseError` if the content cannot be parsed for some reason
    public static func parse<S: ToRustStr>(_ str: S) throws -> Document {
        try str.toRustStr({ rustStr -> Result<Document, ParseError> in
            let errorPtr = UnsafeMutableRawPointer.allocate(byteCount: MemoryLayout<_RustString>.stride, alignment: MemoryLayout<_RustString>.alignment).assumingMemoryBound(to: _RustString.self)
            let result = __liveview_native_core$Document$parse(rustStr.toFfiRepr(), errorPtr)
            if result.is_ok {
                errorPtr.deallocate()
                let doc = Document(__Document(ptr: result.ok_result))
                return .success(doc)
            } else {
                let rustString = RustString(errorPtr.move())
                return .failure(ParseError(message: rustString.toString()))
            }
                      }).get()
    }

    /// Renders this document to a `String` for display and comparison
    public func toString() -> String {
        let str = RustString(__liveview_native_core$Document$to_string(self.repr))
        return str.toString()
    }

    /// Register a callback to be fired when a matching event occurs on this document.
    ///
    /// The given callback receives the document to which the event applies, as well as the
    /// (optional) context object provided.
    ///
    /// Only one callback per event type is supported. Calling this function multiple times for the
    /// same event will only result in the last callback provided being invoked for that tevent
    ///
    /// - Parameters:
    ///   - event: The `EventType` for which the given callback should be invoked
    ///   - context: A caller-provided value which should be passed to the callback when it is invoked
    ///   - callback: The callback to invoke when an event of the given type occurs
    ///
    public func on(_ event: EventType, _ callback: @escaping (Document, AnyObject?) -> ()) {
        self.handlers[event] = Handler(context: nil, callback: callback)
    }

    public func on<T: AnyObject>(_ event: EventType, with context: T, _ callback: @escaping (Document, AnyObject?) -> ()) {
        self.handlers[event] = Handler(context: context, callback: callback)
    }

    /// Updates this document by calculating the changes needed to make it equivalent to `doc`,
    /// and then applying those changes.
    ///
    /// - Parameters:
    ///   - doc: The document to compare against
    public func merge(with doc: Document) {
        if __liveview_native_core$Document$merge(self.repr, doc.repr) {
            if let handler = self.handlers[.changed] {
                handler.call(self)
            }
        }
    }

    /// Returns a reference to the root node of the document
    ///
    /// The root node is not part of the document itself, but can be used to traverse the document tree top-to-bottom.
    public func root() -> NodeRef {
        return __liveview_native_core$Document$root(self.repr)
    }

    /// Enables indexing of the document by node reference, returning the reified `Node` to which it corresponds
    public subscript(ref: NodeRef) -> Node {
        let node = __liveview_native_core$Document$get(self.repr, ref)
        return Node(doc: self, ref: ref, data: node)
    }

    func getChildren(_ ref: NodeRef) -> RustSlice<NodeRef> {
        let slice = __liveview_native_core$Document$children(self.repr, ref)
        return RustSlice(ptr: slice.start, len: Int(slice.len))
    }

    func getAttrs(_ ref: NodeRef) -> RustSlice<AttributeRef> {
        let slice = __liveview_native_core$Document$attributes(self.repr, ref)
        return RustSlice(ptr: slice.start, len: Int(slice.len))
    }

    func getAttr(_ ref: AttributeRef) -> Attribute {
        let attribute = __liveview_native_core$Document$get_attribute(self.repr, ref)
        return Attribute(attribute)
    }
}

/// Represents a node in the document tree
///
/// A node can be one of three types:
///
/// - A root node, which is a special marker node for the root of the document
/// - A leaf node, which is simply text content, cannot have children or attributes
/// - An element node, which can have children and attributes
///
/// A node in a document is uniquely identified by a `NodeRef` for the lifetime of
/// that node in the document. But a `NodeRef` is not a stable identifier when the
/// tree is modified. In some cases the `NodeRef` remains the same while the content
/// changes, and in others, a new node is allocated, so a new `NodeRef` is used.
public class Node: Identifiable {
    /// The type and associated data of this node
    public enum Data {
        case root
        case element(ElementNode)
        case leaf(String)
    }

    let doc: Document

    /// The identifier for this node in its `Document`
    public let id: NodeRef
    /// The type and data associated with this node
    public let data: Data
    /// The attributes associated with this node
    public lazy var attributes: [Attribute] = {
        let refs = doc.getAttrs(id)
        var attributes: [Attribute] = []
        for ref in refs {
            attributes.append(doc.getAttr(ref))
        }
        return attributes
    }()

    init(doc: Document, ref: NodeRef, data: __Node) {
        self.id = ref
        self.doc = doc
        switch data.ty {
        case .NodeTypeRoot:
            self.data = .root
        case .NodeTypeElement:
            self.data = .element(ElementNode(doc: doc, ref: ref, data: data.data.element))
        case .NodeTypeLeaf:
            self.data = .leaf(RustStr(data.data.leaf).toString()!)
        }
    }

    /// Nodes are indexable by attribute name, returning the first attribute with that name
    public subscript(_ name: AttributeName) -> Attribute? {
        for attr in attributes {
            if attr.name == name {
                return attr
            }
        }
        return nil
    }
}
extension Node: Sequence {
    public func makeIterator() -> NodeChildrenIterator {
        let children = self.doc.getChildren(self.id)
        return NodeChildrenIterator(doc: self.doc, children: children)
    }
}

/// An iterator for the direct children of a `Node`
public struct NodeChildrenIterator: IteratorProtocol {
    let doc: Document
    let children: RustSlice<NodeRef>
    var index: Int = 0

    init(doc: Document, children: RustSlice<NodeRef>) {
        self.doc = doc
        self.children = children
    }

    public mutating func next() -> Node? {
        if index >= children.len {
            return nil
        }
        let child = doc[children[index]]
        index += 1
        return child
    }
}

/// Represents a node in a `Document` which can have children and attributes
public struct ElementNode {
    /// An (optional) namespace for the element tag name
    public let namespace: String?
    /// The name of the element tag in the document
    public let tag: String
    /// A dictionary of attributes associated with this element
    public let attributes: [AttributeName: String]

    init(doc: Document, ref: NodeRef, data: __Element) {
        self.namespace = RustStr(data.ns).toString()
        self.tag = RustStr(data.tag).toString()!
        let attrs = RustSlice<AttributeRef>(data.attributes)
        var attributes = Dictionary<AttributeName, String>(minimumCapacity: attrs.len)
        for attr in attrs {
            let attribute = doc.getAttr(attr)
            attributes[attribute.name] = attribute.value
        }
        self.attributes = attributes
    }
}

/// An attribute is a named string value associated with an element
public struct Attribute {
    /// The fully-qualified name of the attribute
    public var name: AttributeName
    /// The value of this attribute, if there was one
    public var value: String?

    init(name: AttributeName, value: String?) {
        self.name = name
        self.value = value
    }

    init(_ attribute: __Attribute) {
        let name = AttributeName(namespace: attribute.ns, name: attribute.name)
        let value = RustStr(attribute.value).toString()
        self.init(name: name, value: value)
    }
}
extension Attribute: Identifiable {
    public var id: AttributeName {
        name
    }
}
extension Attribute: Equatable {
    public static func == (lhs: Attribute, rhs: Attribute) -> Bool {
        return lhs.name == rhs.name && lhs.value == rhs.value
    }
}
extension Attribute: Hashable {
    public func hash(into hasher: inout Hasher) {
        hasher.combine(name)
        hasher.combine(value)
    }
}

/// Represents a fully-qualified attribute name
///
/// Attribute names can be namespaced, so rather than represent them as a plain `String`,
/// we use this type to preserve the information for easy accessibility.
public struct AttributeName {
    public var namespace: String?
    public var name: String

    public init(_ name: String) {
        self.init(namespace: nil, name: name)
    }

    public init(namespace: String?, name: String) {
        self.namespace = namespace
        self.name = name
    }

    init(namespace: _RustStr, name: _RustStr) {
        let ns = RustStr(namespace)
        let n = RustStr(name)
        if ns.isEmpty {
            self.init(namespace: nil, name: n.toString()!)
        } else {
            self.init(namespace: ns.toString(), name: n.toString()!)
        }
    }
}
extension AttributeName: Identifiable {
    public var id: String {
        if let ns = namespace {
            return "\(ns):\(name)"
        } else {
            return name
        }
    }
}
extension AttributeName: Equatable {
    public static func == (lhs: AttributeName, rhs: AttributeName) -> Bool {
        return lhs.namespace == rhs.namespace && lhs.name == rhs.name
    }
}
extension AttributeName: Comparable {
    public static func < (lhs: AttributeName, rhs: AttributeName) -> Bool {
        // Both namespaces are nil, then compare by name
        if lhs.namespace == nil && rhs.namespace == nil {
            return lhs.name < rhs.name
        }
        // Neither namespace are nil, compare by namespace, then by name
        if let lhsNs = lhs.namespace, let rhsNs = rhs.namespace {
            if lhsNs != rhsNs {
                return lhsNs < rhsNs
            } else {
                return lhs.name < rhs.name
            }
        }
        // Otherwise, one of the namespaces are nil, and nil namespaces always come first
        return lhs.namespace == nil
    }
}
extension AttributeName: Hashable {
    public func hash(into hasher: inout Hasher) {
        hasher.combine(namespace)
        hasher.combine(name)
    }
}

import LiveViewNativeCore

public struct ParseError: Error {
    let message: String

    init(message: String) {
        self.message = message
    }
}

public class Document {
    var repr: __Document

    init(_ doc: __Document) {
        self.repr = doc
    }

    deinit {
        __liveview_native_core$Document$drop(self.repr)
    }
}

extension Document {
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

    public func merge(with doc: Document) -> Bool {
        return __liveview_native_core$Document$merge(self.repr, doc.repr)
    }

    public func root() -> NodeRef {
        return __liveview_native_core$Document$root(self.repr)
    }

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

public class Node: Identifiable {
    public enum Data {
        case Root
        case Element(HTMLElement)
        case Leaf(String)
    }

    let doc: Document
    public let id: NodeRef
    public let data: Data
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
            self.data = .Root
        case .NodeTypeElement:
            self.data = .Element(HTMLElement(doc: doc, ref: ref, data: data.data.element))
        case .NodeTypeLeaf:
            self.data = .Leaf(RustStr(data.data.leaf).toString()!)
        }
    }

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

public struct HTMLElement {
    public let namespace: String?
    public let tag: String
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

public struct Attribute {
    public var name: AttributeName
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


/// Represents a node in a `Document` which can have children and attributes
public struct ElementData {
    /// An (optional) namespace for the element tag name
    public let namespace: String?
    /// The name of the element tag in the document
    public let tag: String
    /// An array of attributes associated with this element
    public let attributes: [Attribute]

    init(doc: Document, ref: NodeRef, data: Element) {

        self.namespace = data.name.namespace
        self.tag = data.name.name
        self.attributes = data.attributes
    }
}

extension AttributeName: ExpressibleByStringLiteral {
    public init(stringLiteral value: String) {
        self.init(rawValue: value)!
    }
}

extension AttributeName {
    /// Creates a name by parsing a string, extracting a namespace if present.
    ///
    /// Fails if the string is empty or there are more than two colon-delimited parts.
    public init?(rawValue: String) {
        let parts = rawValue.split(separator: ":")
        switch parts.count {
        case 1:
            self.name = rawValue
        case 2:
            self.namespace = String(parts[0])
            self.name = String(parts[1])
        default:
            return nil
        }
    }
}

/*
/// A sequence representing the direct children of a node.
public struct NodeChildrenSequence: Sequence, Collection, RandomAccessCollection {
    public typealias Element = Node
    public typealias Index = Int

    let doc: Document
    let slice: [NodeRef]
    public let startIndex: Int
    public let endIndex: Int

    public func index(after i: Int) -> Int {
        i + 1
    }

    public subscript(position: Int) -> Node {
        doc[slice[startIndex + position]]
    }
}

/// A sequence of the recursive children of a node, visited in depth-first order.
///
/// See ``Node/depthFirstChildren()``
public struct NodeDepthFirstChildrenSequence: Sequence {
    public typealias Element = Node

    let root: Node

    public func makeIterator() -> Iterator {
        return Iterator(children: [root.children().makeIterator()])
    }

    public struct Iterator: IteratorProtocol {
        public typealias Element = Node

        var children: [NodeChildrenSequence.Iterator]

        public mutating func next() -> Node? {
            if !children.isEmpty {
                if let node = children[children.count - 1].next() {
                    children.append(node.children().makeIterator())
                    return node
                } else {
                    children.removeLast()
                    return self.next()
                }
            } else {
                return nil
            }
        }
    }
}
*/

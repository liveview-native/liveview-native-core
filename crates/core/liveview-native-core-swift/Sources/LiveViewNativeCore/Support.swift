import Foundation
class SimpleHandler: DocumentChangeHandler {
    func handle(_ context: String, _ changeType: ChangeType, _ nodeRef: NodeRef, _ parent: NodeRef?) {
    }
}
public enum EventType {
    /// When a document is modified in some way, the `changed` event is raised
    case changed
}

//public typealias Payload = [String: Any]
extension Document {
    public subscript(ref: NodeRef) -> Node {
        let data = self.get(ref)
        return Node(self, ref, data)
    }
    public static func parseFragmentJson(payload: [String: Any]) throws -> Document {
        let jsonData = try JSONSerialization.data(withJSONObject: payload, options: .prettyPrinted)
        let payload = String(data: jsonData, encoding: .utf8)!
        return try Document.parseFragmentJson(payload)
    }
    public func mergeFragmentJson(
        _ payload: [String: Any]
        //_ callback: @escaping (Document, NodeRef) -> ()
        ) throws {
        let jsonData = try JSONSerialization.data(withJSONObject: payload, options: .prettyPrinted)
        let payload = String(data: jsonData, encoding: .utf8)!

        // TODO: Fix this
        let simple = SimpleHandler()
        return try self.mergeFragmentJson(payload, simple)
    }

    // TODO: Fix this
    public func on(_ event: EventType, _ callback: @escaping (Document, NodeRef) -> ()) {

        //precondition(!self.handlers.keys.contains(event))
        //self.handlers[event] = callback
    }
}

extension AttributeName: ExpressibleByStringLiteral {
    public init(stringLiteral value: String) {
        self.init(namespace: .none, name: value)
    }
    public var rawValue: String {
        if let namespace {
            return "\(namespace):\(name)"
        } else {
            return name
        }
    }
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
extension Node {
    public func children() -> NodeChildrenSequence {
        let children = self.getChildren()
        return NodeChildrenSequence(doc: self.document(), slice: children, startIndex: children.startIndex, endIndex: children.endIndex)
    }
    public func depthFirstChildren() -> NodeDepthFirstChildrenSequence {
        return NodeDepthFirstChildrenSequence(root: self)
    }
    public subscript(_ name: AttributeName) -> Attribute? {
        let attributes = self.attributes()
        return attributes.first { $0.name == name }
    }
}
extension NodeRef: Hashable {
    public static func == (lhs: NodeRef, rhs: NodeRef) -> Bool {
        return lhs.ref() == rhs.ref()
    }
    public func hash(into hasher: inout Hasher) {
        hasher.combine(ref())
    }
}

public struct NodeChildrenSequence: Sequence, Collection, RandomAccessCollection {
    //public typealias Element = Node
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
public struct NodeDepthFirstChildrenSequence: Sequence {
    public typealias Element = Node

    let root: Node

    public func makeIterator() -> Iterator {
        //return Iterator(children: [root.children().makeIterator()])
        return Iterator(children: [])
    }

    public struct Iterator: IteratorProtocol {
        public typealias Element = Node

        var children: [NodeChildrenSequence.Iterator]

        public mutating func next() -> Node? {
            if !children.isEmpty {
                if let node = children[children.count - 1].next() {
                    //children.append(node.children().makeIterator())
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

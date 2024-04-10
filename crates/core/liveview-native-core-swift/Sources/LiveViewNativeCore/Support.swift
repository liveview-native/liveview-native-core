import Foundation

final class SimpleHandler: DocumentChangeHandler {
    let callback: (NodeRef, NodeData, NodeRef?) -> ()
    init (
        _ callback: @escaping (NodeRef, NodeData, NodeRef?) -> ()
    ) {
       self.callback = callback
    }
    func handle(_ changeType: ChangeType, _ node: NodeRef, _ data: NodeData, _ parent: NodeRef?) {
        switch changeType {
        case .add:
            self.callback(parent!, data, parent)
        case .remove:
            self.callback(parent!, data, parent)
        case .change:
            self.callback(node, data, parent)
        case .replace:
            self.callback(parent!, data, parent)
        }
    }

}

extension Document {
    public subscript(ref: NodeRef) -> Node {
        let data = self.get(ref)
        return Node(self, ref, data)
    }
    public static func parseFragmentJson(payload: [String: Any]) throws -> Document {
        let jsonData = try JSONSerialization.data(withJSONObject: payload)
        let payload = String(data: jsonData, encoding: .utf8)!
        return try Document.parseFragmentJson(payload)
    }
    public func mergeFragmentJson(
        _ payload: [String: Any]
        ) throws {
        let jsonData = try JSONSerialization.data(withJSONObject: payload)
        let payload = String(data: jsonData, encoding: .utf8)!

        return try self.mergeFragmentJson(payload)
    }

    public func on(_ event: EventType, _ callback: @escaping (NodeRef, NodeData, NodeRef?) -> ()) {

        let simple = SimpleHandler(callback)
        self.setEventHandler(simple)
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
        return NodeChildrenSequence(doc: self.document(), slice: children)
    }
    public func depthFirstChildren() -> NodeDepthFirstChildrenSequence {
        return NodeDepthFirstChildrenSequence(root: self)
    }
    public subscript(_ name: AttributeName) -> Attribute? {
        let attributes = self.attributes()
        return attributes.first { $0.name == name }
    }
    public func toString() -> String {
        return self.display()
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
    public typealias Element = Node
    public typealias Index = Int

    let doc: Document
    let slice: [NodeRef]
    public var startIndex: Int { 0 }

    public var endIndex: Int { self.slice.count }

    public func index(after i: Int) -> Int {
        i + 1
    }
    public subscript(position: Int) -> Node {
        return doc[slice[startIndex + position]]
    }
}
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

import Foundation

final class SimpleHandler: DocumentChangeHandler {
    let callback: (NodeRef, NodeData, NodeRef?) -> Void

    init(
        _ callback: @escaping (NodeRef, NodeData, NodeRef?) -> Void
    ) {
        self.callback = callback
    }

    func handleDocumentChange(
        _ changeType: ChangeType, _ node: NodeRef, _ data: NodeData, _ parent: NodeRef?
    ) {
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

    func handleChannelStatus(_ channelStatus: LiveChannelStatus) -> ControlFlow {
        switch channelStatus {
        case .joined,
            .joining,
            .leaving,
            .shuttingDown,
            .waitingForSocketToConnect,
            .waitingToJoin,
            .waitingToRejoin:
            return .continueListening
        case .left,
            .shutDown:
            return .exitOk
        }
    }

}

extension Document {
    public subscript(ref: NodeRef) -> Node {
        return self.getNode(ref)
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

    public func on(_ event: EventType, _ callback: @escaping (NodeRef, NodeData, NodeRef?) -> Void)
    {

        let simple = SimpleHandler(callback)
        self.setEventHandler(simple)
    }
    public func toString() -> String {
        return self.render()
    }
}

extension AttributeName: ExpressibleByStringLiteral {
    public init(stringLiteral value: String) {
        self.init(namespace: .none, name: value)
    }
    public init(name: String) {
        self.init(namespace: .none, name: name)
    }
    public var rawValue: String {
        if let namespace {
            return "\(namespace):\(name)"
        } else {
            return name
        }
    }
    public init?(rawValue: String) {
        let parts = rawValue.split(separator: ":" as Character)
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
        return NodeChildrenSequence(slice: self.getChildren())
    }
    public func depthFirstChildren() -> NodeDepthFirstChildrenSequence {
        return NodeDepthFirstChildrenSequence(slice: self.getDepthFirstChildren())
    }
    public func depthFirstChildrenOriginal() -> NodeDepthFirstChildrenSequenceOriginal {
        return NodeDepthFirstChildrenSequenceOriginal(root: self)
    }
    public subscript(_ name: AttributeName) -> Attribute? {
        return self.getAttribute(name)
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

    let slice: [Node]
    public var startIndex: Int { self.slice.startIndex }

    public var endIndex: Int { self.slice.endIndex }

    public func index(after i: Int) -> Int {
        slice.index(after: i)
    }
    public subscript(position: Int) -> Node {
        return slice[startIndex + position]
    }
}
public struct NodeDepthFirstChildrenSequence: Sequence, Collection, RandomAccessCollection {
    public typealias Element = Node
    public typealias Index = Int

    let slice: [Node]
    public var startIndex: Int { self.slice.startIndex }

    public var endIndex: Int { self.slice.endIndex }

    public func index(after i: Int) -> Int {
        slice.index(after: i)
    }
    public subscript(position: Int) -> Node {
        return slice[startIndex + position]
    }
}

public struct NodeDepthFirstChildrenSequenceOriginal: Sequence {
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

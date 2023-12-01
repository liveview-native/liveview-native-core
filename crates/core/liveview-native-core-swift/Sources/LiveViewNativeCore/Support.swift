/*
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
*/

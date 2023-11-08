
/// Represents a node in a `Document` which can have children and attributes
public struct ElementData {
    /// An (optional) namespace for the element tag name
    public let namespace: String?
    /// The name of the element tag in the document
    public let tag: String
    /// An array of attributes associated with this element
    public let attributes: [Attribute]

    /*
    init(doc: Document, ref: NodeRef, data: __Element) {
        self.namespace = RustStr(data.ns).toString()
        self.tag = RustStr(data.tag).toString()!
        let av = AttributeVec(data.attributes)
        self.attributes = av.toSlice().map { attr in Attribute(attr) }
    }
    */
}

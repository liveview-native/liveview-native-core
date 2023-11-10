package org.phoenixframework.liveview.lib

/** This class represents the valid node types of a `Document` tree */
sealed class Node {

    /**
     * A marker node that indicates the root of a document A document may only have a single root,
     * and it has no attributes
     */
    object Root : Node()

    /** A typed node that can carry attributes and may contain other nodes */
    class Element internal constructor(pointer: Long) : Node() {
        private val nativeObject: Long = pointer

        val namespace: String
            get() = get_namespace(nativeObject)

        val tag: String
            get() = get_tag(nativeObject)

        val attributes: Array<Attribute>
            get() = get_attributes(nativeObject)

        private external fun get_attributes(element: Long): Array<Attribute>

        private external fun get_tag(element: Long): String

        private external fun get_namespace(element: Long): String

        private external fun drop(pointer: Long)

        protected fun finalize() {
            drop(nativeObject)
        }
    }

    /**
     * A leaf node is an untyped node, typically text, and does not have any attributes or children
     */
    data class Leaf(val value: String) : Node()
}
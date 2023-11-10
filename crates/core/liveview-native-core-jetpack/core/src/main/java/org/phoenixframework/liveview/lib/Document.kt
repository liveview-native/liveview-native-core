package org.phoenixframework.liveview.lib
import android.util.Log;
import uniffi.LiveViewNativeCore.Document
//import org.phoenixframework.liveview_native_core.LiveViewNativeCore


class Document {
    private var nativeObject: Long
    private val asBorrow: Boolean

    constructor() {
        nativeObject = empty()
        asBorrow = false
    }

    internal constructor(pointer: Long, borrowed: Boolean = true ) {
        nativeObject = pointer
        asBorrow = borrowed
    }

    companion object {
        init {
            System.loadLibrary("liveview_native_core")
        }

        /**
         * Parses a `Document` from a string
         *
         * @throws Exception if the document is malformed
         */
        @Throws
        fun parse(string: String): Document {
            val result = JavaResult(do_parse(string))
            return result.document ?: throw Exception(result.error)
        }

        private external fun do_parse(text: String): Long

        /** Output logs from the Rust side into android's logcat */
        private external fun initialize_log()

        enum class ChangeType {
            Change,
            Add,
            Remove,
            Replace
        }

        open class Handler {
            private fun ffiOnHandle(context: Long, changeType: Byte, nodeRef: Int, parent: Int) {
                onHandle(
                    Document(context, true),
                    ChangeType.values()[changeType.toInt()],
                    NodeRef(nodeRef),
                    if (parent == 0) null else NodeRef(parent))
            }

            open fun onHandle(
                context: Document,
                changeType: ChangeType,
                nodeRef: NodeRef,
                parent: NodeRef?
            ) {}
        }
    }

    fun getNodeString(nodeRef: NodeRef): String = node_to_string(nativeObject, nodeRef.ref)

    /**
     * Returns the root node of the document The root node can be used in insertion operations, but
     * can not have attributes applied to it
     */
    // could also use lazy
    val rootNodeRef
        get() = run { NodeRef(root(nativeObject)) }

    /** Returns the data associated with the given `NodeRef` */
    fun getNode(nodeRef: NodeRef): Node {
        val nodePtr = get_node(nativeObject, nodeRef.ref)
        // construct node
        return when (val nodeType = get_node_type(nodePtr)) {
            0.toByte() -> {
                Node.Root
            }
            1.toByte() -> {
                val elementPtr = get_node_element(nodePtr)
                Node.Element(elementPtr)
            }
            2.toByte() -> {
                Node.Leaf(get_node_leaf_string(nativeObject, nodeRef.ref))
            }
            else -> throw Exception("Unknown node type ${nodeType}")
        }
    }

    /** Returns the children of `node` as a string */
    fun getChildren(nodeRef: NodeRef) = get_children(nativeObject, nodeRef.ref).map { NodeRef(it) }

    /** Returns the parent of `node`, if it has one */
    fun getParent(nodeRef: NodeRef) =
        get_parent(nativeObject, nodeRef.ref).let { if (it < 0) null else NodeRef(it) }

    fun merge(other: Document, handler: Handler) {
        merge(nativeObject, other.nativeObject, handler)
    }

    /**
     * Deserializes the json, renders it, parses it and then diffs it against
     * the current document.
     *
     * @throws Exception if the this is not deserializable, not renderable or
     * if the rendering is unparsable.
     */
    @Throws
    fun mergeFragmentJson(other_json: String, handler: Handler) {
        merge_fragment_json(nativeObject, other_json, handler)
    }

    private external fun merge(doc: Long, other: Long, handler: Handler)

    private external fun merge_fragment_json(doc: Long, other_json: String, handler: Handler)

    private external fun get_parent(doc: Long, nodeRef: Int): Int

    private external fun get_children(doc: Long, nodeRef: Int): IntArray

    private external fun get_node_leaf_string(doc: Long, nodeRef: Int): String

    private external fun get_node_element(node: Long): Long

    private external fun get_node_type(node: Long): Byte

    private external fun get_node(doc: Long, node: Int): Long

    private external fun node_to_string(doc: Long, node: Int): String

    private external fun root(doc: Long): Int

    private external fun empty(): Long

    private external fun do_to_string(pointer: Long): String

    private external fun drop(pointer: Long)

    override fun toString(): String = do_to_string(nativeObject)

    protected fun finalize() {
        if (!asBorrow) {
            drop(nativeObject)
        }
    }
}

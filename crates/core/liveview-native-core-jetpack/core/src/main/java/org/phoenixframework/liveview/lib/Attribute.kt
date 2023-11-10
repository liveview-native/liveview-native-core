package org.phoenixframework.liveview.lib

class Attribute internal constructor(private var nativeObject: Long) {

    val name: String
        get() = get_name(nativeObject)

    /** The namespace of an attribute */
    val namespace: String
        get() = get_namespace(nativeObject)

    val value: String
        get() = get_value(nativeObject)

    private external fun get_name(pointer: Long): String

    private external fun get_value(pointer: Long): String

    private external fun get_namespace(pointer: Long): String

    override fun toString(): String =
        "Attribute {\n" +
                "  Name: ${name.ifEmpty { "None" }}\n" +
                "  Namespace: ${namespace.ifEmpty { "None" }}\n" +
                "  Value: ${value.ifEmpty { "None" }}\n" +
                "}"

    protected fun finalize() {
        drop(nativeObject)
    }

    private external fun drop(pointer: Long)
}

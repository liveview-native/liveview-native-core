#pragma once

#include "Support.h"

typedef uint32_t NodeRef;
typedef uint32_t AttributeRef;

typedef struct __Document {
  void *ptr;
} __Document;

typedef struct __Element {
  _RustStr ns;
  _RustStr tag;
  _RustSlice attributes;
} __Element;

typedef struct __Attribute {
  _RustStr ns;
  _RustStr name;
  _RustStr value;
} __Attribute;

enum __NodeType {
  NodeTypeRoot = 0,
  NodeTypeElement = 1,
  NodeTypeLeaf = 2,
} __attribute__((enum_extensibility(closed)));

typedef enum __NodeType __NodeType;

typedef union __NodeData {
  void *root;
  __Element element;
  _RustStr leaf;
} __NodeData;

typedef struct __Node {
  __NodeType ty;
  __NodeData data;
} __Node;

extern __Document __liveview_native_core$Document$empty();

extern void __liveview_native_core$Document$drop(__Document doc);

extern _RustResult __liveview_native_core$Document$parse(_RustStr text,
                                                         _RustString *error);

extern bool __liveview_native_core$Document$merge(__Document doc,
                                                  __Document other);

extern NodeRef __liveview_native_core$Document$root(__Document doc);

extern __Node __liveview_native_core$Document$get(__Document doc, NodeRef node);

extern _RustSlice __liveview_native_core$Document$children(__Document doc,
                                                           NodeRef node);

extern _RustSlice __liveview_native_core$Document$attributes(__Document doc,
                                                             NodeRef node);

extern __Attribute
__liveview_native_core$Document$get_attribute(__Document doc,
                                              AttributeRef attr);

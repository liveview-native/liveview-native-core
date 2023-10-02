#pragma once

#include "Support.h"

typedef uint32_t NodeRef;

typedef struct _AttributeVec {
  const void *start;
  uintptr_t len;
  uintptr_t capacity;
} _AttributeVec;

typedef struct __Document {
  void *ptr;
} __Document;

typedef struct __Element {
  _RustStr ns;
  _RustStr tag;
  _AttributeVec attributes;
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

typedef struct __OptionNodeRef {
	bool is_some;
	NodeRef some_value;
} __OptionNodeRef;

enum __ChangeType {
  ChangeTypeChange = 0,
  ChangeTypeAdd = 1,
  ChangeTypeRemove = 2,
  ChangeTypeReplace = 3
} __attribute__((enum_extensibility(closed)));

typedef enum __ChangeType __ChangeType;

extern void __liveview_native_core$AttributeVec$drop(_AttributeVec vec);

extern _RustString __liveview_native_core$Document$to_string(__Document doc);

extern _RustString
__liveview_native_core$Document$node_to_string(__Document doc, NodeRef node);

extern __Document __liveview_native_core$Document$empty();

extern void __liveview_native_core$Document$drop(__Document doc);

extern _RustResult __liveview_native_core$Document$parse(_RustStr text,
                                                         _RustString *error);

typedef void (*OnChangeCallback)(void *context, __ChangeType type, NodeRef node, __OptionNodeRef parent);

extern void __liveview_native_core$Document$merge(__Document doc,
                                                  __Document other,
                                                  OnChangeCallback callback,
                                                  void *context);

extern _RustResult __liveview_native_core$Document$merge_fragment_json(__Document doc,
                                                  _RustStr json,
                                                  OnChangeCallback callback,
                                                  void *context,
                                                  _RustString *error
                                                  );

extern NodeRef __liveview_native_core$Document$root(__Document doc);

extern __Node __liveview_native_core$Document$get(__Document doc, NodeRef node);

extern _RustSlice __liveview_native_core$Document$children(__Document doc,
                                                           NodeRef node);

extern _AttributeVec __liveview_native_core$Document$attributes(__Document doc,
                                                                NodeRef node);

extern __OptionNodeRef __liveview_native_core$Document$get_parent(__Document doc, NodeRef node);

#pragma once

#include <stdbool.h>

#if defined(_WIN32)
typedef unsigned char uint8_t;
typedef uint8_t u_int8_t;
typedef unsigned short uint16_t;
typedef uint16_t u_int16_t;
typedef unsigned uint32_t;
typedef uint32_t u_int32_t;
typedef unsigned long long uint64_t;
typedef uint64_t u_int64_t;
#else
#include <stdint.h>
#endif

typedef struct _RustResult {
  bool is_ok;
  void *ok_result;
} _RustResult;

typedef struct _RustSlice {
  const void *start;
  uintptr_t len;
} _RustSlice;

typedef struct _RustStr {
  const void *start;
  uintptr_t len;
} _RustStr;

extern bool __liveview_native_core$RustStr$eq(_RustStr lhs, _RustStr rhs);
extern bool __liveview_native_core$RustStr$lt(_RustStr lhs, _RustStr rhs);

typedef struct _RustString {
  const void *start;
  uintptr_t len;
  uintptr_t capacity;
} _RustString;

extern void __liveview_native_core$RustString$drop(_RustString string);

typedef struct _RustVec {
  const void *start;
  uintptr_t len;
  uintptr_t capacity;
} _RustVec;

extern void __liveview_native_core$RustVec$Attribute$drop(_RustVec vec);

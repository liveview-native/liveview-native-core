import Foundation
import LiveViewNativeCore


public class RustStr {
    let ptr: UnsafeRawPointer?
    public let len: Int

    public var isEmpty: Bool { return len == 0 }

    public init(_ raw: _RustStr) {
        self.len = Int(raw.len)
        if self.len == 0 {
            self.ptr = nil
        } else {
            self.ptr = raw.start
        }
    }

    public init(ptr: UnsafeRawPointer?, len: Int) {
        self.ptr = ptr
        self.len = len
    }

    public func toString() -> String? {
        if len == 0 {
            return nil
        } else {
            let bytes = self.toBufferPointer()
            return String(bytes: bytes, encoding: .utf8)
        }
    }

    public func asBytes() -> RustSlice<UInt8> {
        RustSlice(ptr: self.ptr, len: self.len)
    }

    func toBufferPointer() -> UnsafeBufferPointer<UInt8> {
        return UnsafeBufferPointer(start: self.ptr.map { $0.assumingMemoryBound(to: UInt8.self) }, count: self.len)
    }

    func toFfiRepr() -> _RustStr {
        return _RustStr(start: self.ptr, len: UInt(self.len))
    }
}
extension RustStr: Identifiable {
    public var id: String {
        self.toString() ?? ""
    }
}
extension RustStr: Equatable {
    public static func == (lhs: RustStr, rhs: RustStr) -> Bool {
        __liveview_native_core$RustStr$eq(lhs.toFfiRepr(), rhs.toFfiRepr())
    }
}
extension RustStr: Comparable {
    public static func < (lhs: RustStr, rhs: RustStr) -> Bool {
        __liveview_native_core$RustStr$lt(lhs.toFfiRepr(), rhs.toFfiRepr())
    }
}
extension RustStr: Hashable {
    public func hash(into hasher: inout Hasher) {
        hasher.combine(bytes: UnsafeRawBufferPointer(start: self.ptr, count: self.len))
    }
}
extension RustStr: Sequence {
    public func makeIterator() -> RustStrIterator {
        return RustStrIterator(self)
    }
}

public struct RustStrIterator: IteratorProtocol {
    var iter: RustSliceIterator<UInt8>
    var decoder: Unicode.UTF8
    var done = false

    init(_ str: RustStr) {
        self.iter = RustSliceIterator(str.asBytes())
        self.decoder = Unicode.UTF8()
    }

    public mutating func next() -> Character? {
        if done {
            return nil
        }

        switch decoder.decode(&iter) {
        case .scalarValue(let v):
            return Character(v)
        case .emptyInput:
            done = true
            return nil
        case .error:
            done = true
            return nil
        }
    }
}

public protocol ToRustStr {
    func toRustStr<T> (_ withUnsafeRustStr: (RustStr) -> T) -> T;
}
extension RustStr: ToRustStr {
    public func toRustStr<T> (_ withUnsafeRustStr: (RustStr) -> T) -> T {
        return withUnsafeRustStr(self)
    }
}
extension String: ToRustStr {
    public func toRustStr<T> (_ withUnsafeRustStr: (RustStr) -> T) -> T {
        return self.utf8CString.withUnsafeBufferPointer({ bufferPtr in
                                                            let rustStr = RustStr(
                                                              ptr: bufferPtr.baseAddress.map { UnsafeRawPointer($0) },
                                                              // Subtract 1 because of the null termination character at the end
                                                              len: bufferPtr.count - 1
                                                            )
                                                            return withUnsafeRustStr(rustStr)
                                                        })
    }
}

public class RustString {
    let ptr: UnsafeRawPointer
    public let len: Int
    public let capacity: Int

    init(_ raw: _RustString) {
        self.ptr = UnsafeRawPointer(raw.start!)
        self.len = Int(raw.len)
        self.capacity = Int(raw.capacity)
    }

    deinit {
        __liveview_native_core$RustString$drop(_RustString(start: self.ptr, len: UInt(self.len), capacity: UInt(self.capacity)))
    }

    public func toString() -> String {
        let bytes = self.toBufferPointer()
        return String(bytes: bytes, encoding: .utf8)!
    }

    func toRustStr() -> RustStr {
        return RustStr(ptr: self.ptr, len: self.len)
    }

    func toBufferPointer() -> UnsafeBufferPointer<UInt8> {
        return UnsafeBufferPointer(start: self.ptr.assumingMemoryBound(to: UInt8.self), count: self.len)
    }
}
extension RustString: Equatable {
    public static func == (lhs: RustString, rhs: RustString) -> Bool {
        return lhs.toRustStr() == rhs.toRustStr()
    }
}
extension RustString: Comparable {
    public static func < (lhs: RustString, rhs: RustString) -> Bool {
        __liveview_native_core$RustStr$lt(lhs.toRustStr().toFfiRepr(), rhs.toRustStr().toFfiRepr())
    }
}
extension RustString: Hashable {
    public func hash(into hasher: inout Hasher) {
        hasher.combine(bytes: UnsafeRawBufferPointer(start: self.ptr, count: self.len))
    }
}

public class RustSlice<T> {
    let ptr: UnsafeRawPointer?
    public let len: Int

    public var isEmpty: Bool { return len == 0 }

    init(ptr: UnsafeRawPointer?, len: Int) {
        self.ptr = ptr
        self.len = len
    }

    convenience init(_ slice: _RustSlice) {
        self.init(ptr: slice.start, len: Int(slice.len))
    }

    public func get(index: Int) -> T? {
        if index >= self.len {
            return nil
        } else {
            return self.toBufferPointer()[index]
        }
    }

    func toBufferPointer() -> UnsafeBufferPointer<T> {
        UnsafeBufferPointer(start: self.ptr.map { $0.assumingMemoryBound(to: T.self) }, count: self.len)
    }
}
extension RustSlice: Equatable where T: Equatable {
    public static func == (lhs: RustSlice<T>, rhs: RustSlice<T>) -> Bool {
        if lhs.len != rhs.len {
            return false
        }
        var li = lhs.makeIterator()
        var ri = rhs.makeIterator()
        while let l = li.next(), let r = ri.next() {
            if l != r {
                return false
            }
        }
        return true
    }
}
extension RustSlice: Sequence {
    public func makeIterator() -> RustSliceIterator<T> {
        return RustSliceIterator(self)
    }
}
extension RustSlice: Collection {
    public typealias Index = Int

    public func index(after i: Int) -> Int {
        i + 1
    }

    public subscript(position: Int) -> T {
        self.get(index: position)!
    }

    public var startIndex: Int { 0 }

    public var endIndex: Int { self.len }
}
extension RustSlice: RandomAccessCollection {}

public struct RustSliceIterator<T>: IteratorProtocol {
    var slice: RustSlice<T>
    var index: Int = 0

    init(_ slice: RustSlice<T>) {
        self.slice = slice
    }

    public mutating func next() -> T? {
        let result = self.slice.get(index: self.index)
        self.index += 1
        return result
    }
}

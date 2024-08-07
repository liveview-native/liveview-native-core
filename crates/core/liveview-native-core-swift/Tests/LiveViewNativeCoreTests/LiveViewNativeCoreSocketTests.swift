import XCTest
@testable import LiveViewNativeCore
#if canImport(SystemConfiguration)
    import SystemConfiguration
#endif

let timeout = TimeInterval(30.0)

let connect_url = "http://127.0.0.1:4001/hello?_format=swiftui";
final class LiveViewNativeCoreSocketTests: XCTestCase {
    func testConnect() async throws {
        let live_socket = try await LiveSocket(connect_url, timeout)
        let _ = try await live_socket.joinLiveviewChannel()
    }

    func testStatus() async throws {
        let live_socket = try await LiveSocket(connect_url, timeout)
        let _ = try await live_socket.joinLiveviewChannel()
        let socket = live_socket.socket()

        var status = socket.status()
        XCTAssertEqual(status, .connected)

        try await socket.disconnect()
        status = socket.status()
        XCTAssertEqual(status, .disconnected)

        try await socket.shutdown()
        status = socket.status()
        XCTAssertEqual(status, .shutDown)
    }
}

// This is a PNG located at crates/core/tests/support/tinycross.png
let base64TileImg = "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABmJLR0QA/wD/AP+gvaeTAAAACXBIWXMAAAsTAAALEwEAmpwYAAAAB3RJTUUH4gEdFQog0ycfAgAAAIJJREFUOMulU0EOwCAIK2T/f/LYwWAAgZGtJzS1BbVEuEVAAACCQOsKlkOrEicwgeVz5tC5R1yrDdnKuo6j6J5ydgd+npOUHfaGEJkQq+6cQNVqP1oQiCJxvAjGT3Dn3l1sKpAdfhPhqXP5xDYLXz7SkYUuUNnrcBWULkRlFqZxtvwH8zGCEN6LErUAAAAASUVORK5CYII="

let upload_url = "http://127.0.0.1:4001/upload?_format=swiftui";
final class LiveViewNativeCoreUploadTests: XCTestCase {
    func testUpload() async throws {
        let live_socket = try await LiveSocket(upload_url, timeout)
        let live_channel = try await live_socket.joinLiveviewChannel()

        let phx_id = try live_channel.getPhxRefFromUploadJoinPayload()
        let image : Data! = Data(base64Encoded: base64TileImg)

        let live_file = LiveFile(image, "png", "foobar.png", phx_id)
        try await live_channel.uploadFile(live_file)
    }
}

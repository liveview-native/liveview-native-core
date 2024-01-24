import XCTest
@testable import LiveViewNativeCore
#if canImport(SystemConfiguration)
    import SystemConfiguration
#endif

let url = "http://127.0.0.1:4000/upload?_lvn[format]=swiftui";

let timeout = TimeInterval(10.0)


final class LiveViewNativeCoreSocketTests: XCTestCase {
    func testConnect() async throws {
        let live_socket = try LiveSocket(url, timeout)
        let lvn_channel = try await live_socket.joinLiveviewChannel()
    }

    func testStatus() async throws {
        let live_socket = try LiveSocket(url, timeout)
        let lvn_channel = try await live_socket.joinLiveviewChannel()
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

import XCTest

@testable import LiveViewNativeCore

#if canImport(SystemConfiguration)
    import SystemConfiguration
#endif

let timeout = TimeInterval(30.0)

let connect_url = "http://127.0.0.1:4001/hello"
final class LiveViewNativeCoreSocketTests: XCTestCase {
    func testConnect() async throws {
        let live_socket = try await LiveSocket(connect_url, "swiftui", .none)
        let _ = try await live_socket.joinLiveviewChannel(.none, .none)
    }

    func testConnectWithOpts() async throws {
        let headers = [String: String]()
        let options = ConnectOpts(headers: headers)
        let live_socket = try await LiveSocket(connect_url, "swiftui", options)
        let _ = try await live_socket.joinLiveviewChannel(.none, .none)
    }

    func testStatus() async throws {
        let live_socket = try await LiveSocket(connect_url, "swiftui", .none)
        let _ = try await live_socket.joinLiveviewChannel(.none, .none)
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
let base64TileImg =
    "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABmJLR0QA/wD/AP+gvaeTAAAACXBIWXMAAAsTAAALEwEAmpwYAAAAB3RJTUUH4gEdFQog0ycfAgAAAIJJREFUOMulU0EOwCAIK2T/f/LYwWAAgZGtJzS1BbVEuEVAAACCQOsKlkOrEicwgeVz5tC5R1yrDdnKuo6j6J5ydgd+npOUHfaGEJkQq+6cQNVqP1oQiCJxvAjGT3Dn3l1sKpAdfhPhqXP5xDYLXz7SkYUuUNnrcBWULkRlFqZxtvwH8zGCEN6LErUAAAAASUVORK5CYII="

let upload_url = "http://127.0.0.1:4001/upload"
final class LiveViewNativeCoreUploadTests: XCTestCase {
    func testUpload() async throws {
        let live_socket = try await LiveSocket(upload_url, "swiftui", .none)
        let live_channel = try await live_socket.joinLiveviewChannel(.none, .none)

        let image: Data! = Data(base64Encoded: base64TileImg)

        let phx_id: String! = try live_channel.getPhxUploadId("avatar")
        let live_file = LiveFile(image, "image/png", "avatar", "foobar.png", phx_id)
        try await live_channel.uploadFile(live_file)
    }
}

// Test basic navigation flow with LiveSocket
func testBasicNavFlow() async throws {
    let url = "http://127.0.0.1:4001/nav/first_page"
    let secondUrl = "http://127.0.0.1:4001/nav/second_page"

    let liveSocket = try await LiveSocket(url, "swiftui", .none)
    let liveChannel = try await liveSocket.joinLiveviewChannel(.none, .none)

    let doc = liveChannel.document()

    let expectedFirstDoc = """
        <Group id="flash-group" />
        <VStack>
            <Text>
                first_page
            </Text>
            <NavigationLink id="Next" destination="/nav/next">
                <Text>
                    NEXT
                </Text>
            </NavigationLink>
        </VStack>
        """

    let exp = try Document.parse(expectedFirstDoc)

    XCTAssertEqual(doc.render(), exp.render())

    let secondChannel = try await liveSocket.navigate(secondUrl, liveChannel, NavOptions())

    let secondDoc = secondChannel.document()

    let expectedSecondDoc = """
        <Group id="flash-group" />
        <VStack>
            <Text>
               second_page
            </Text>
            <NavigationLink id="Next" destination="/nav/next">
                <Text>
                    NEXT
                </Text>
            </NavigationLink>
        </VStack>
        """

    let secondExp = try Document.parse(expectedSecondDoc)

    XCTAssertEqual(secondDoc.render(), secondExp.render())
}

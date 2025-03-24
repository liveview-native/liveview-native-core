import XCTest

@testable import LiveViewNativeCore

#if canImport(SystemConfiguration)
    import SystemConfiguration
#endif

let timeout = TimeInterval(30.0)

func withTimeoutOf<T>(seconds: Double, operation: @escaping () async throws -> T) async throws -> T
{
    try await withThrowingTaskGroup(of: T.self) { group in
        group.addTask {
            try await operation()
        }

        group.addTask {
            // Convert to nanoseconds internally
            try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
            throw TimeoutError(seconds: seconds)
        }

        do {
            if let result = try await group.next() {
                group.cancelAll()
                return result
            }
        } catch {
            group.cancelAll()
            throw error
        }

        throw TimeoutError(seconds: seconds)
    }
}

struct TimeoutError: Error {
    let seconds: Double

    var localizedDescription: String {
        return "Operation timed out after \(seconds) seconds"
    }
}

let connect_url = "http://127.0.0.1:4001/hello"
final class LiveViewNativeCoreSocketTests: XCTestCase {
    func testConnect() async throws {
        let live_socket = try await LiveSocket(connect_url, "swiftui", .none, .none)
        let _ = try await live_socket.joinLiveviewChannel(.none, .none)
    }

    func testConnectWithOpts() async throws {
        let headers = [String: String]()
        let options = ConnectOpts(headers: headers)
        let live_socket = try await LiveSocket(connect_url, "swiftui", options, .none)
        let _ = try await live_socket.joinLiveviewChannel(.none, .none)
    }

    func testStatus() async throws {
        let live_socket = try await LiveSocket(connect_url, "swiftui", .none, .none)
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

    func testBasicConnection() async throws {
        let builder = LiveViewClientBuilder()
        let client = try await builder.connect("http://127.0.0.1:4001/hello", ClientConnectOpts())

        let expected = """
            <Group id="flash-group" />
            <VStack>
                <Text>
                    Hello SwiftUI!
                </Text>
            </VStack>
            """

        let exp = try Document.parse(expected)
        XCTAssertEqual(try client.document().render(), exp.render())

    }

    func testNavigation() async throws {
        let builder = LiveViewClientBuilder()
        builder.setLogLevel(.debug)
        let client = try await builder.connect(
            "http://127.0.0.1:4001/nav/first_page", ClientConnectOpts())

        let watcher = client.statusStream()

        let initialDoc = try client.document()
        let expectedInitial = """
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
        let expInitial = try Document.parse(expectedInitial)
        XCTAssertEqual(initialDoc.render(), expInitial.render())

        let secondPageId = try await client.navigate(
            "http://127.0.0.1:4001/nav/second_page", NavOptions())

        let postNavStatus = try await withTimeoutOf(seconds: 5) {
            await watcher.nextStatus()
        }

        guard case .connected(let postNavChannelStatus) = postNavStatus else {
            fatalError(
                "Expected client to remain connected after navigation, but got \(postNavStatus)")
        }

        guard case .connected(let documentTwo) = postNavChannelStatus else {
            fatalError(
                "Expected channel to remain connected after navigation, but got \(postNavChannelStatus)"
            )
        }

        // document should change.
        let secondDoc = try client.document()
        let expectedSecond = """
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
        let expSecond = try Document.parse(expectedSecond)
        XCTAssertEqual(documentTwo.render(), expSecond.render())
        XCTAssertEqual(secondDoc.render(), expSecond.render())
    }
}

// This is a PNG located at crates/core/tests/support/tinycross.png
let base64TileImg =
    "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABmJLR0QA/wD/AP+gvaeTAAAACXBIWXMAAAsTAAALEwEAmpwYAAAAB3RJTUUH4gEdFQog0ycfAgAAAIJJREFUOMulU0EOwCAIK2T/f/LYwWAAgZGtJzS1BbVEuEVAAACCQOsKlkOrEicwgeVz5tC5R1yrDdnKuo6j6J5ydgd+npOUHfaGEJkQq+6cQNVqP1oQiCJxvAjGT3Dn3l1sKpAdfhPhqXP5xDYLXz7SkYUuUNnrcBWULkRlFqZxtvwH8zGCEN6LErUAAAAASUVORK5CYII="

let upload_url = "http://127.0.0.1:4001/upload"
final class LiveViewNativeCoreUploadTests: XCTestCase {
    func testUpload() async throws {
        let live_socket = try await LiveSocket(upload_url, "swiftui", .none, .none)
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

    let liveSocket = try await LiveSocket(url, "swiftui", .none, .none)
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

    let secondChannel = try await liveSocket.navigate(secondUrl, .none, NavOptions())

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

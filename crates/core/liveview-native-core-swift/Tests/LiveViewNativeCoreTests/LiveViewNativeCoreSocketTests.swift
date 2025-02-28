import XCTest

@testable import LiveViewNativeCore

#if canImport(SystemConfiguration)
    import SystemConfiguration
#endif

let timeout = TimeInterval(30.0)

let connect_url = "http://127.0.0.1:4001/hello"
final class LiveViewNativeCoreSocketTests: XCTestCase {
    func testConnect() async throws {
        let builder = LiveViewClientBuilder()
        let client = try await builder.connect(connect_url, ClientConnectOpts())
    }

    func testConnectWithOpts() async throws {
        let headers = [String: String]()
        let options = ClientConnectOpts(headers: headers)
        let builder = LiveViewClientBuilder()
        let client = try await builder.connect(connect_url, options)
    }

    func testStatus() async throws {
        let builder = LiveViewClientBuilder()
        let client = try await builder.connect(connect_url, ClientConnectOpts())

        var status = try client.status()
        XCTAssertEqual(status, .connected)

        try await client.disconnect()
        status = try client.status()
        XCTAssertEqual(status, .disconnected)
    }

    func testBasicConnection() async throws {
        let builder = LiveViewClientBuilder()
        let client = try await builder.connect("http://127.0.0.1:4001/hello", ClientConnectOpts())
        let document = try client.document()

        let expected = """
            <Group id="flash-group" />
            <VStack>
                <Text>
                    Hello SwiftUI!
                </Text>
            </VStack>
            """

        let exp = try Document.parse(expected)
        XCTAssertEqual(document.render(), exp.render())
    }

    func testNavigation() async throws {
        let builder = LiveViewClientBuilder()
        let client = try await builder.connect(
            "http://127.0.0.1:4001/nav/first_page", ClientConnectOpts())

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

        // document should change.
        // TODO: validate doc change is sent in event loop
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
        XCTAssertEqual(secondDoc.render(), expSecond.render())

    }
}

// This is a PNG located at crates/core/tests/support/tinycross.png
let base64TileImg =
    "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABmJLR0QA/wD/AP+gvaeTAAAACXBIWXMAAAsTAAALEwEAmpwYAAAAB3RJTUUH4gEdFQog0ycfAgAAAIJJREFUOMulU0EOwCAIK2T/f/LYwWAAgZGtJzS1BbVEuEVAAACCQOsKlkOrEicwgeVz5tC5R1yrDdnKuo6j6J5ydgd+npOUHfaGEJkQq+6cQNVqP1oQiCJxvAjGT3Dn3l1sKpAdfhPhqXP5xDYLXz7SkYUuUNnrcBWULkRlFqZxtvwH8zGCEN6LErUAAAAASUVORK5CYII="

let upload_url = "http://127.0.0.1:4001/upload"
final class LiveViewNativeCoreUploadTests: XCTestCase {
    func testUpload() async throws {
        // Using the new LiveViewClient API
        let builder = LiveViewClientBuilder()
        let client = try await builder.connect(upload_url, ClientConnectOpts())

        let image: Data! = Data(base64Encoded: base64TileImg)

        let phx_id: String! = try client.getPhxUploadId("avatar")
        let live_file = LiveFile(image, "image/png", "avatar", "foobar.png", phx_id)
        try await client.uploadFiles([live_file])
    }
}

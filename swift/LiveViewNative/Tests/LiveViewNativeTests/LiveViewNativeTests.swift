import XCTest
@testable import LiveViewNative

class MyContext {
    var didChange = false
}

final class LiveViewNativeTests: XCTestCase {
    func testIntegration() throws {
        let context = MyContext()
        let input = """
<html lang="en">
    <head>
        <meta charset="utf-8" />
    </head>
    <body class="new-value" class="main">
        some content
    </body>
</html>
"""
        let doc1 = try Document.parse(input)
        doc1.on(.changed, with: context) { doc, ctx in
            XCTAssertEqual(ctx is MyContext, true)
            (ctx as! MyContext).didChange = true
        }

        let updated = """
<html lang="en">
    <head>
        <meta charset="utf-8" />
        <meta name="title" content="Hello World" />
    </head>
    <body class="new-value" class="main">
        new content
    </body>
</html>
"""
        let doc2 = try Document.parse(updated)

        doc1.merge(with: doc2)

        XCTAssertEqual(context.didChange, true)
    }
}

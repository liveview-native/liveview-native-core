import XCTest
@testable import LiveViewNativeCore

class MyContext {
    var didChange = false
}
class SimpleHandler: DocumentChangeHandler {
    func handle(context: String, changeType: ChangeType, nodeRef: NodeRef, optionNodeRef: NodeRef?) {
    }
}

final class LiveViewNativeCoreTests: XCTestCase {
    func testIntegration() throws {
        let context = MyContext()
        let input = """
        <html lang="en">
            <head>
                <meta charset="utf-8" />
            </head>
            <body foo="new-value" bar="main">
                some content
            </body>
        </html>
        """
        let doc1 = try Document.parse(input: input)
/*
        doc1.on(.changed) { doc, _ in
            context.didChange = true
        }
        */
        let rendered1 = doc1.render()
        XCTAssertEqual(input, rendered1)

        let updated = """
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="title" content="Hello World" />
            </head>
            <body foo="new-value" bar="main">
                new content
            </body>
        </html>
        """
        let doc2 = try Document.parse(input: updated)
        let rendered2 = doc2.render()
        XCTAssertEqual(updated, rendered2)
        /*

        doc1.merge(with: doc2)

        XCTAssertEqual(context.didChange, true)

        let finalRender = doc1.render()
        XCTAssertEqual(finalRender, rendered2)
*/
    }
    func testJsonIntegration() throws {
        let context = MyContext()
        let initial_json = """
        {
          "0":"0",
          "1":"0",
          "2":"",
          "s":[
            "<Column>\\n  <Button phx-click=\\"inc\\">\\n    <Text>Increment</Text>\\n  </Button>\\n  <Button phx-click=\\"dec\\">\\n    <Text>Decrement</Text>\\n  </Button>\\n  <Text>Static Text </Text>\\n  <Text>Counter 1: ",
            " </Text>\\n  <Text>Counter 2: ",
            " </Text>\\n",
            "\\n</Column>"
            ]
        }
        """
        let initial_document = try Document.parseFragmentJson(input: initial_json)
        let initial_rendered = initial_document.render()
        var expected = """
        <Column>
            <Button phx-click="inc">
                <Text>
                    Increment
                </Text>
            </Button>
            <Button phx-click="dec">
                <Text>
                    Decrement
                </Text>
            </Button>
            <Text>
                Static Text
            </Text>
            <Text>
                Counter 1: 0
            </Text>
            <Text>
                Counter 2: 0
            </Text>
        </Column>
        """
        XCTAssertEqual(expected, initial_rendered)
        let first_increment = """
        {
          "0":"1",
          "1":"1",
          "2":{
            "0":{
              "s":[
                "\\n      <Text fontWeight=\\"W600\\" fontSize=\\"24\\">Item ",
                "!!!</Text>\\n",
                "\\n",
                "\\n"
              ],
              "p":{
                 "0":[

                   "\\n        <Text color=\\" #FFFF0000\\">Number = ",

                   " + 3 is even</Text>\\n"
                 ],
                 "1":[
                   "\\n        <Text>Number + 4 = ",
                   " is odd</Text>\\n"
                   ]
              },
              "d":[["1",{"0":"1","s":0},{"0":"5","s":1}]]
            },
            "1":"101",
            "s":[
              "\\n",
              "\\n    <Text>Number + 100 is ","</Text>\\n"
            ]
          }
        }
        """
        let simple = SimpleHandler()
        try initial_document.mergeFragmentJson(json: first_increment, handler: simple)
        let second_render = initial_document.render()
        expected = """
        <Column>
            <Button phx-click="inc">
                <Text>
                    Increment
                </Text>
            </Button>
            <Button phx-click="dec">
                <Text>
                    Decrement
                </Text>
            </Button>
            <Text>
                Static Text
            </Text>
            <Text>
                Counter 1: 1
            </Text>
            <Text>
                Counter 2: 1
            </Text>
            <Text fontWeight="W600" fontSize="24">
                Item 1!!!
            </Text>
            <Text color=" #FFFF0000">
                Number = 1 + 3 is even
            </Text>
            <Text>
                Number + 4 = 5 is odd
            </Text>
            <Text>
                Number + 100 is 101
            </Text>
        </Column>
        """
        XCTAssertEqual(expected, second_render)
        let second_increment = """
        {
          "0":"2",
          "1":"2",
          "2":{
            "0":{
              "p":{
                "0":[
                  "\\n        <Text color=\\" #FFFF0000\\">Number = ",
                  " + 3 is even</Text>\\n"
                ],
                "1":[
                  "\\n        <Text>Number + 4 = ",
                  " is odd</Text>\\n"
                ],
                "2":[
                  "\\n        <Text color=\\" #FF0000FF\\">Number = ",
                  " + 3 is odd</Text>\\n"
                ],
                "3":[
                  "\\n        <Text>Number + 4 = ",
                  " is even</Text>\\n"
                ]
              },
              "d":[
                ["1",{"0":"1","s":0},{"0":"5","s":1}],
                ["2",{"0":"2","s":2},{"0":"6","s":3}]
              ]
            },
            "1":"102"
          }
        }
        """
        try initial_document.mergeFragmentJson(json: second_increment, handler: simple)
        let third_render = initial_document.render()
        expected = """
        <Column>
            <Button phx-click="inc">
                <Text>
                    Increment
                </Text>
            </Button>
            <Button phx-click="dec">
                <Text>
                    Decrement
                </Text>
            </Button>
            <Text>
                Static Text
            </Text>
            <Text>
                Counter 1: 2
            </Text>
            <Text>
                Counter 2: 2
            </Text>
            <Text fontWeight="W600" fontSize="24">
                Item 1!!!
            </Text>
            <Text color=" #FFFF0000">
                Number = 1 + 3 is even
            </Text>
            <Text>
                Number + 4 = 5 is odd
            </Text>
            <Text fontWeight="W600" fontSize="24">
                Item 2!!!
            </Text>
            <Text color=" #FF0000FF">
                Number = 2 + 3 is odd
            </Text>
            <Text>
                Number + 4 = 6 is even
            </Text>
            <Text>
                Number + 100 is 102
            </Text>
        </Column>
        """
        XCTAssertEqual(expected, third_render)
    }

    /*
    func testDepthFirstIterator() throws {
        let input = "<a><b><c /></b><d /></a>"
        let doc = try Document.parse(input)
        let rootNode = doc[doc.root()]
        let tags = rootNode.depthFirstChildren().map {
            if case .element(let data) = $0.data {
                return data.tag
            } else {
                fatalError()
            }
        }
        XCTAssertEqual(tags, ["a", "b", "c", "d"])
    }

    func testNodeToString() throws {
        let input = "<a><b>hello</b></a>"
        let doc = try Document.parse(input)
        let a = doc[doc.root()].children().first!
        let b = a.children().first!
        XCTAssertEqual(b.render(), "<b>\n    hello\n</b>")
    }

    func testUppercaseTags() throws {
        let input = "<A>test</a>"
        let doc = try Document.parse(input)
        let a = doc[doc.root()].children().first!
        guard case .element(let data) = a.data else {
            XCTFail()
            return
        }
        XCTAssertEqual(data.tag, "A")
    }

    func testTagSwap() throws {
        let doc1 = try Document.parse("<a /><b />")
        let b = doc1[doc1.root()].children()[1]

        let doc2 = try Document.parse("<b />")

        doc1.merge(with: doc2)

        // ensure the element is preserved across updates.
        XCTAssertEqual(doc1[doc1.root()].children()[0].id, b.id)
    }

    func testReplace() throws {
        let doc1 = try Document.parse("<a />")
        let a = doc1[doc1.root()].children()[0]

        let doc2 = try Document.parse("<b />")

        doc1.merge(with: doc2)

        XCTAssertEqual(doc1[doc1.root()].children()[0].id, a.id)
    }
    */
}

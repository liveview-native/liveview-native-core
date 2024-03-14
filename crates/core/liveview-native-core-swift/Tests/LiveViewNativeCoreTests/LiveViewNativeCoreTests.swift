import XCTest
@testable import LiveViewNativeCore
class SimpleHandler: DocumentChangeHandler {
    func handle(_ doc: Document, _ changeType: ChangeType, _ nodeRef: NodeRef, _ parent: NodeRef?) {
    }
}

final class LiveViewNativeCoreTests: XCTestCase {
    func testIntegration() throws {
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
        let doc1 = try Document.parse(input)
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
        let doc2 = try Document.parse(updated)
        let rendered2 = doc2.render()
        XCTAssertEqual(updated, rendered2)
    }
    func testJsonIntegration() throws {
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
        let simple = SimpleHandler()
        let initial_document = try Document.parseFragmentJson(initial_json)
        initial_document.setEventHandler(simple)
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
        try initial_document.mergeFragmentJson(first_increment)
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
        try initial_document.mergeFragmentJson(second_increment)
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
}

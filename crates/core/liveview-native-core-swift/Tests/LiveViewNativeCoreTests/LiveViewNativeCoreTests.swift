import XCTest

@testable import LiveViewNativeCore

final class SimpleHandler: DocumentChangeHandler {
    func handleDocumentChange(
        _ changeType: ChangeType, _ nodeRef: NodeRef, _ nodeData: NodeData, _ parent: NodeRef?
    ) {
        print("Handler:", changeType, ", node:", nodeRef.ref())
    }

    func handleChannelStatus(_ channelStatus: LiveChannelStatus) -> ControlFlow {
        return .continueListening
    }
}
final class LiveViewNativeTreeTests: XCTestCase {
    func testDepthFirstTraversal() throws {
        let input = """
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
        let doc = try Document.parse(input)
        let root = doc[doc.root()]
        let old_depth_first = root.depthFirstChildrenOriginal()
        let new_depth_first = root.depthFirstChildren()
        for (new, old) in zip(new_depth_first, old_depth_first) {
            XCTAssertEqual(new.id.ref(), old.id.ref())
        }
    }
}

final class LiveViewNativeCoreTests: XCTestCase {
    func testForSwiftUIClientBug() throws {
        let initial_json = """
            {
                "s" : [
                    "",
                    ""
                ],
                "0" : {
                    "0" : "",
                    "s" : [
                        "<VStack>\\n  ",
                        "\\n  <Button phx-click=\\"inc_temperature\\"> Increment Temperature </Button>\\n  <Button phx-click=\\"dec_temperature\\"> Decrement Temperature </Button>\\n</VStack>"
                    ],
                    "r" : 1
                }
            }
            """
        let doc = try Document.parseFragmentJson(initial_json)
        let simple = SimpleHandler()
        doc.setEventHandler(simple)
        print("initial:\n", doc.render())
        var expected = """
            <VStack>
                <Button phx-click="inc_temperature">
                    Increment Temperature
                </Button>
                <Button phx-click="dec_temperature">
                    Decrement Temperature
                </Button>
            </VStack>
            """
        XCTAssertEqual(expected, doc.render())

        let first_increment = """
            {
                "0" : {
                    "0" : {
                        "s" : [
                            "<Text> Temperature: ",
                            " </Text>"
                        ],
                        "d" : [
                            ["Increment"]
                        ]
                    }
                }
            }
            """
        try doc.mergeFragmentJson(first_increment)
        expected = """
            <VStack>
                <Text>
                    Temperature: Increment
                </Text>
                <Button phx-click="inc_temperature">
                    Increment Temperature
                </Button>
                <Button phx-click="dec_temperature">
                    Decrement Temperature
                </Button>
            </VStack>
            """
        print("first:\n", doc.render())
        XCTAssertEqual(expected, doc.render())
        let second_increment = """
            {
                "0" : {
                    "0" : {
                        "d" : [       ]
                    }
                }
            }
            """
        try doc.mergeFragmentJson(second_increment)
        print("second:\n", doc.render())
        let third_increment = """
            {   "0" : {     "0" : {       "d" : [         [           "Increment"         ]       ]     }   } }
            """
        try doc.mergeFragmentJson(third_increment)
        print("third:\n", doc.render())
    }
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
        let doc = try Document.parseFragmentJson(initial_json)
        doc.setEventHandler(simple)
        let initial_rendered = doc.render()
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
        try doc.mergeFragmentJson(first_increment)
        let second_render = doc.render()
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
        try doc.mergeFragmentJson(second_increment)
        let third_render = doc.render()
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

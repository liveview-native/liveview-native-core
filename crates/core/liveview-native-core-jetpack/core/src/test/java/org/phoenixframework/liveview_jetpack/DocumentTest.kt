

import org.junit.Test
import org.junit.Assert.assertEquals;
import org.phoenixframework.liveview.lib.Document;
import org.phoenixframework.liveview.lib.DocumentChangeHandler;
import org.phoenixframework.liveview.lib.ChangeType;
import org.phoenixframework.liveview.lib.NodeRef;

class SimpleChangeHandler: DocumentChangeHandler {
    constructor() {
    }

    override fun `handle`(
        `context`: String,
        `changeType`: ChangeType,
        `nodeRef`: NodeRef,
        `optionNodeRef`: NodeRef?,
    ) {
        println("${changeType}")
    }
}

class DocumentTest {

    @Test
    fun document_parse() {
        // The formatting of this multi line string is very specific such that it matches the expected output.
        var input = """<VStack modifiers="">
    <VStack>
        <LiveForm id="login" phx-submit="login">
            <TextField name="email" modifiers="">
                Email
            </TextField>
            <LiveSubmitButton modifiers="">
                <Text>
                    Enter
                </Text>
            </LiveSubmitButton>
        </LiveForm>
    </VStack>
</VStack>"""
        var doc = Document.parse(input);
        var rendered = doc.render();
        assertEquals(input, rendered)
    }

    @Test
    fun json_merging() {
        var input = """
        {
          "0":"0",
          "1":"0",
          "2":"",
          "s":[
            "<Column>\n  <Button phx-click=\"inc\">\n    <Text>Increment</Text>\n  </Button>\n  <Button phx-click=\"dec\">\n    <Text>Decrement</Text>\n  </Button>\n  <Text>Static Text </Text>\n  <Text>Counter 1: ",
            " </Text>\n  <Text>Counter 2: ",
            " </Text>\n",
            "\n</Column>"
            ]
        }
        """
        var doc = Document.parseFragmentJson(input)
        var expected = """<Column>
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
</Column>"""
        var rendered = doc.render();
        assertEquals(expected, rendered)
        var first_increment = """{
  "0":"1",
  "1":"1",
  "2":{
    "0":{
      "s":[
        "\n      <Text fontWeight=\"W600\" fontSize=\"24\">Item ",
        "!!!</Text>\n",
        "\n",
        "\n"
      ],
      "p":{
         "0":[

           "\n        <Text color=\" #FFFF0000\">Number = ",

           " + 3 is even</Text>\n"
         ],
         "1":[
           "\n        <Text>Number + 4 = ",
           " is odd</Text>\n"
           ]
      },
      "d":[["1",{"0":"1","s":0},{"0":"5","s":1}]]
    },
    "1":"101",
    "s":[
      "\n",
      "\n    <Text>Number + 100 is ","</Text>\n"
    ]
  }
}
        """
        var simple = SimpleChangeHandler()
        doc.mergeFragmentJson(first_increment, simple);
        rendered = doc.render();
        expected = """<Column>
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
</Column>"""
        assertEquals(expected, rendered)
    }
}

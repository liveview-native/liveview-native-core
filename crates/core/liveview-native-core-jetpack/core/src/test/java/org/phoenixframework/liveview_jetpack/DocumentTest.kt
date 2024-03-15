

import org.junit.Test
import org.junit.Assert.assertEquals;
import org.phoenixframework.liveviewnative.core.Document;
import org.phoenixframework.liveviewnative.core.DocumentChangeHandler;
import org.phoenixframework.liveviewnative.core.ChangeType;
import org.phoenixframework.liveviewnative.core.NodeRef;
import org.phoenixframework.liveviewnative.core.LiveSocket;
import org.phoenixframework.liveviewnative.core.LiveFile;

import java.time.Duration;
import kotlinx.coroutines.*;
import kotlin.coroutines.*;
import kotlinx.coroutines.test.runTest;
import kotlin.system.*;
import java.util.Base64;

class SocketTest {
    @Test
    fun simple_connect() = runTest {
        var live_socket = LiveSocket("http://127.0.0.1:4000/upload?_lvn[format]=swiftui", Duration.ofDays(10));
        var live_channel = live_socket.joinLiveviewChannel()
        var phx_id = live_channel.getPhxRefFromUploadJoinPayload()
        // This is a PNG located at crates/core/tests/support/tinycross.png
        var base64TileImg = "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABmJLR0QA/wD/AP+gvaeTAAAACXBIWXMAAAsTAAALEwEAmpwYAAAAB3RJTUUH4gEdFQog0ycfAgAAAIJJREFUOMulU0EOwCAIK2T/f/LYwWAAgZGtJzS1BbVEuEVAAACCQOsKlkOrEicwgeVz5tC5R1yrDdnKuo6j6J5ydgd+npOUHfaGEJkQq+6cQNVqP1oQiCJxvAjGT3Dn3l1sKpAdfhPhqXP5xDYLXz7SkYUuUNnrcBWULkRlFqZxtvwH8zGCEN6LErUAAAAASUVORK5CYII="

        val contents = Base64.getDecoder().decode(base64TileImg)
        var live_file = LiveFile(contents, "png", "foobar.png", phx_id)
        live_channel.uploadFile(live_file)
    }
}

class SimpleChangeHandler: DocumentChangeHandler {
    constructor() {
    }

    override fun `handle`(
        `context`: Document,
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
        var simple = SimpleChangeHandler();
        doc.setEventHandler(simple);
        doc.mergeFragmentJson(first_increment);
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

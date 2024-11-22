import java.util.Base64
import kotlin.coroutines.*
import kotlin.system.*
import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test
import org.phoenixframework.liveviewnative.core.ChangeType
import org.phoenixframework.liveviewnative.core.ConnectOpts
import org.phoenixframework.liveviewnative.core.Document
import org.phoenixframework.liveviewnative.core.DocumentChangeHandler
import org.phoenixframework.liveviewnative.core.LiveFile
import org.phoenixframework.liveviewnative.core.LiveSocket
import org.phoenixframework.liveviewnative.core.NavOptions
import org.phoenixframework.liveviewnative.core.NodeData
import org.phoenixframework.liveviewnative.core.NodeRef

class SocketTest {
    @Test
    fun simple_connect() = runTest {
        var live_socket = LiveSocket.connect("http://127.0.0.1:4001/upload", "jetpack", null)
        var live_channel = live_socket.joinLiveviewChannel(null, null)
        // This is a PNG located at crates/core/tests/support/tinycross.png
        var base64TileImg =
                "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABmJLR0QA/wD/AP+gvaeTAAAACXBIWXMAAAsTAAALEwEAmpwYAAAAB3RJTUUH4gEdFQog0ycfAgAAAIJJREFUOMulU0EOwCAIK2T/f/LYwWAAgZGtJzS1BbVEuEVAAACCQOsKlkOrEicwgeVz5tC5R1yrDdnKuo6j6J5ydgd+npOUHfaGEJkQq+6cQNVqP1oQiCJxvAjGT3Dn3l1sKpAdfhPhqXP5xDYLXz7SkYUuUNnrcBWULkRlFqZxtvwH8zGCEN6LErUAAAAASUVORK5CYII="

        val contents = Base64.getDecoder().decode(base64TileImg)
        val phx_upload_id = live_channel.getPhxUploadId("avatar")
        var live_file = LiveFile(contents, "image/png", "avatar", "foobar.png", phx_upload_id)
        live_channel.uploadFile(live_file)
    }
}

class SocketTestOpts {
    @Test
    fun connect_with_opts() = runTest {
        var opts = ConnectOpts()
        var live_socket = LiveSocket.connect("http://127.0.0.1:4001/upload", "jetpack", opts)
        var live_channel = live_socket.joinLiveviewChannel(null, null)

        // This is a PNG located at crates/core/tests/support/tinycross.png
        var base64TileImg =
                "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABmJLR0QA/wD/AP+gvaeTAAAACXBIWXMAAAsTAAALEwEAmpwYAAAAB3RJTUUH4gEdFQog0ycfAgAAAIJJREFUOMulU0EOwCAIK2T/f/LYwWAAgZGtJzS1BbVEuEVAAACCQOsKlkOrEicwgeVz5tC5R1yrDdnKuo6j6J5ydgd+npOUHfaGEJkQq+6cQNVqP1oQiCJxvAjGT3Dn3l1sKpAdfhPhqXP5xDYLXz7SkYUuUNnrcBWULkRlFqZxtvwH8zGCEN6LErUAAAAASUVORK5CYII="

        val contents = Base64.getDecoder().decode(base64TileImg)
        val phx_upload_id = live_channel.getPhxUploadId("avatar")
        var live_file = LiveFile(contents, "image/png", "avatar", "foobar.png", phx_upload_id)
        live_channel.uploadFile(live_file)
    }
}

class SimpleChangeHandler : DocumentChangeHandler {
    constructor() {}

    override fun `handle`(
            `changeType`: ChangeType,
            `nodeRef`: NodeRef,
            `nodeData`: NodeData,
            `optionNodeRef`: NodeRef?,
    ) {
        println("${changeType}")
    }
}

class DocumentTest {

    @Test
    fun document_parse() {
        // The formatting of this multi line string is very specific such that it matches the
        // expected output.
        var input =
                """<VStack modifiers="">
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
        var doc = Document.parse(input)
        var rendered = doc.render()
        assertEquals(input, rendered)
    }
    @Test
    fun json_merging_from_empty() {
        var doc = Document.empty()
        var input =
                """
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
        doc.mergeFragmentJson(input)
        var expected =
                """<Column>
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
        var rendered = doc.render()
        assertEquals(expected, rendered)
    }

    @Test
    fun json_merging() {
        var input =
                """
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
        var expected =
                """<Column>
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
        var rendered = doc.render()
        assertEquals(expected, rendered)
        var first_increment =
                """{
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
        doc.setEventHandler(simple)
        doc.mergeFragmentJson(first_increment)
        rendered = doc.render()
        expected =
                """<Column>
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

    @Test
    fun basic_nav_flow() = runTest {
        val host = "127.0.0.1:4001"
        val url = "http://$host/nav/first_page"

        val liveSocket = LiveSocket.connect(url, "jetpack", null)
        val liveChannel = liveSocket.joinLiveviewChannel(null, null)
        val doc = liveChannel.document()

        val expectedFirstDoc =
                """
               <Box size="fill" background="system-blue">
                 <Text align="Center">
                        first_page
                   <Link destination="/nav/next">
                        <Text class="bold">Next</Text>
                   </Link>
                 </Text>
               </Box>
           """.trimIndent()

        val exp = Document.parse(expectedFirstDoc)
        assertEquals(exp.render(), doc.render())

        val secondUrl = "http://$host/nav/second_page"
        liveSocket.navigate(secondUrl, NavOptions())

        val secondChannel = liveSocket.joinLiveviewChannel(null, null)
        val secondDoc = secondChannel.document()

        val expectedSecondDoc =
                """
                <Box size="fill" background="system-blue">
                  <Text align="Center">
                        second_page
                    <Link destination="/nav/next">
                         <Text class="bold">Next</Text>
                    </Link>
                  </Text>
                </Box>
           """.trimIndent()

        val secondExp = Document.parse(expectedSecondDoc)
        assertEquals(secondExp.render(), secondDoc.render())
    }
}

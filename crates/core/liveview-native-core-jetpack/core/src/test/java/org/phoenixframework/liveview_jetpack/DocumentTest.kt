

import org.junit.Test
import org.junit.Assert.assertEquals;
import org.phoenixframework.liveview_native_core.Document;

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
    }
}



import org.junit.Test
import org.junit.Assert.*
import org.phoenixframework.liveview_native_core.Document;

class DocumentTest {
    /*
    @Test
    fun it_constructs_empty_native_doc() {
        Document()
    }
    */

    @Test
    fun it_morphs_live_form() {
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

        /*

        var to = Document.parse("""
        <VStack modifiers="">
            <VStack>
                <Text>Success! Check your email for magic link</Text>
            </VStack>
        </VStack>
        """);

        doc.merge(to,  Document.Companion.Handler());
        */
    }

}

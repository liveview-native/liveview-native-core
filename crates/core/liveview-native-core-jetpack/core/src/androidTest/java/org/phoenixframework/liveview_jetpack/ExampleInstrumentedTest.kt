package org.phoenixframework.liveview_jetpack

import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.phoenixframework.liveviewnative.core.Document;

import org.junit.Test
import org.junit.runner.RunWith

import org.junit.Assert.*

/**
 * Instrumented test, which will execute on an Android device.
 *
 * See [testing documentation](http://d.android.com/tools/testing).
 */
@RunWith(AndroidJUnit4::class)
class ExampleInstrumentedTest {
    @Test
    fun useAppContext() {
        // Context of the app under test.
        val appContext = InstrumentationRegistry.getInstrumentation().targetContext
        assertEquals("org.phoenixframework.liveview_native_core_jetpack.test", appContext.packageName)
    }

    @Test
    fun parseDocument() {
        var doc = Document.parse("""
        <VStack modifiers="">
            <VStack>
                <LiveForm id="login" phx-submit="login">
                    <TextField name="email" modifiers="">
                        Email
                    </TextField>
                    <LiveSubmitButton modifiers="">
                        <Text>Enter</Text>
                    </LiveSubmitButton>
                </LiveForm>
            </VStack>
        </VStack>
        """);
    }
}

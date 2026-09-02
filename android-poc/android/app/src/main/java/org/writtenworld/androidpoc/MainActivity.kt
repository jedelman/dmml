package org.writtenworld.androidpoc

import android.app.Activity
import android.os.Bundle
import android.widget.TextView

// F1 (jedelman/dmml#1): the entire UI is one TextView showing whatever
// the Haskell side of the bridge returns -- deliberately not more than
// that. This activity's only job is proving `System.loadLibrary` +
// `external fun` can reach a real GHC-compiled function; it is not a
// sketch of the actual game client.
class MainActivity : Activity() {
    companion object {
        init {
            System.loadLibrary("dmmlbridge")
        }
    }

    private external fun greetFromHaskell(): String

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val tv = TextView(this)
        tv.text = greetFromHaskell()
        setContentView(tv)
    }
}

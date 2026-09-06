package org.writtenworld.androidpoc

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface

// F1 follow-up, now Compose: the fake `hsGreet` PoC and the plain-
// TextView follow-up are both gone -- this hosts GameScreen.kt, a real
// touch-only UI (tap a button to fire a transition, no typing, no
// terminal) calling the real interpreter via DmmlBridge (dmml-hs's own
// DMML.Materialize/DMML.Guard/DMML.Fire, through DMML.JniBridge's
// history-aware functions). Same interaction shape
// dmml-hs/app/TouchBrowser.hs already proved out over plain HTTP+links
// on host GHC -- this is that shape again, native, in Compose.
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface {
                    GameScreen()
                }
            }
        }
    }
}

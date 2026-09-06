package org.writtenworld.androidpoc

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import org.json.JSONArray

// Same fixture dmml-hs/app/JniBridgeSmokeTest.hs (host GHC, through the
// exact same FFI-shaped functions) and dmml-hs/examples/
// interactive-browser-demo (the terminal REPL AND TouchBrowser.hs's
// HTTP version) already verified end-to-end -- kept identical on
// purpose, so a real on-device run is checking the exact same
// known-good input every other layer already proved, not a fresh guess.
private const val WORLD_SRC = """commit prospect
  declare relation location
  declare relation name
  declare relation state
  declare relation role

  player/one `location` room/forge
  player/one `state` idle

  npc/smith :: a type/smith
  npc/smith `name` "Tamsin"
  npc/smith `location` room/forge
  npc/smith `role` role/oresmith

  room/forge `name` "the smithy"
"""

private const val MACHINE_SRC = """machine machine/actions

  states
    idle
    working

  transition work()
    idle -> working
    guard self `location` room/forge
    assert working
    retract idle

  transition rest()
    working -> idle
    guard self `location` room/forge
    assert idle
    retract working
"""

private const val SELF_NODE = "player/one"

/**
 * The whole state model is one growing list: the ORIGINAL world commit,
 * then each previously-fired commit's own rendered text, in firing
 * order -- exactly [DMML.JniBridge]'s history contract (see
 * `DmmlBridge.kt`'s own doc comment), which is exactly
 * `TouchBrowser.hs`'s own `[IdentifiedCommit]` history, just held in
 * Compose state and serialized to JSON to cross the JNI boundary
 * instead of accumulated natively in Haskell. Tapping a button is the
 * ENTIRE input surface -- no text field anywhere in this screen.
 */
@Composable
fun GameScreen() {
    var history by remember { mutableStateOf(listOf(WORLD_SRC)) }
    var lastError by remember { mutableStateOf<String?>(null) }

    val historyJson = remember(history) { JSONArray(history).toString() }
    val rendered = remember(historyJson) { DmmlBridge.renderHistory(historyJson, MACHINE_SRC) }
    val actionsRaw = remember(historyJson) { DmmlBridge.actionsHistory(historyJson, MACHINE_SRC, SELF_NODE) }
    // actionsHistory renders "machineNode/transitionIdent" per line, and
    // the machine node itself contains a "/" -- taking the LAST segment
    // is correct regardless, since a real transition ident is always a
    // single, slash-free identifier (DMML.Surface's own grammar).
    val actions = remember(actionsRaw) {
        actionsRaw.lines().filter { it.isNotBlank() }.map { it.substringAfterLast("/") }
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text("dmml -- touch browser", style = MaterialTheme.typography.titleLarge)
        Spacer(Modifier.height(8.dp))

        lastError?.let { err ->
            Text("refused: $err", color = MaterialTheme.colorScheme.error)
            Spacer(Modifier.height(8.dp))
        }

        Text(
            text = rendered,
            fontFamily = FontFamily.Monospace,
            modifier = Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState())
        )

        Spacer(Modifier.height(16.dp))

        if (actions.isEmpty()) {
            Text("Nothing more you can do here.", color = MaterialTheme.colorScheme.onSurfaceVariant)
        } else {
            actions.forEach { transitionIdent ->
                Button(
                    onClick = {
                        val result = DmmlBridge.fireHistory(historyJson, MACHINE_SRC, SELF_NODE, transitionIdent)
                        if (DmmlBridge.isError(result)) {
                            lastError = result.removePrefix("ERROR: ")
                        } else {
                            lastError = null
                            history = history + result
                        }
                    },
                    modifier = Modifier.fillMaxWidth().padding(vertical = 6.dp).heightIn(min = 56.dp)
                ) {
                    Text(transitionIdent, style = MaterialTheme.typography.titleMedium)
                }
            }
        }
    }
}

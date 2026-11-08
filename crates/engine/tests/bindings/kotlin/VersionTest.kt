// Smoke test for the BoltFFI Kotlin binding on the desktop JVM. Run via tests/bindings/run.sh kotlin.
import com.example.quran_muaalem_engine.version

fun main() {
    val v = version()
    check(Regex("""^\d+\.\d+\.\d+""").containsMatchIn(v)) { "version() is not semver: $v" }

    val expected = System.getenv("ENGINE_VERSION")
    if (!expected.isNullOrEmpty()) {
        check(v == expected) { "version() = $v, expected $expected" }
    }

    println("ok: version() = $v")
}

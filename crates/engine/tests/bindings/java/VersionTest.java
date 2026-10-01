import com.example.quran_muaalem_engine.QuranMuaalemEngine;

/** Smoke test for the BoltFFI Java binding. Run via tests/bindings/run.sh java. */
public final class VersionTest {
    public static void main(String[] args) {
        String version = QuranMuaalemEngine.version();

        check(version != null && version.matches("^\\d+\\.\\d+\\.\\d+.*"),
            "version() is not semver: " + version);

        String expected = System.getenv("ENGINE_VERSION");
        if (expected != null && !expected.isEmpty()) {
            check(version.equals(expected),
                "version() = " + version + ", expected " + expected);
        }

        System.out.println("ok: version() = " + version);
    }

    private static void check(boolean condition, String message) {
        if (!condition) {
            System.err.println("FAIL: " + message);
            System.exit(1);
        }
    }
}

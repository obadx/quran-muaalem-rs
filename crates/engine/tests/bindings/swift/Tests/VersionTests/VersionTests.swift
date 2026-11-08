import Foundation
import QuranMuaalemEngine
import XCTest

final class VersionTests: XCTestCase {
    func testVersionMatchesCrate() {
        let version = QuranMuaalemEngine.version()
        XCTAssertNotNil(version.range(of: #"^\d+\.\d+\.\d+"#, options: .regularExpression),
                        "version() is not semver: \(version)")

        if let expected = ProcessInfo.processInfo.environment["ENGINE_VERSION"], !expected.isEmpty {
            XCTAssertEqual(version, expected)
        }
    }
}

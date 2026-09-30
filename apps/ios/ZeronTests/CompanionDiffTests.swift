import XCTest
@testable import Zeron

final class CompanionDiffTests: XCTestCase {
    private let patch = """
    diff --git a/src/main.rs b/src/main.rs
    index 111..222 100644
    --- a/src/main.rs
    +++ b/src/main.rs
    @@ -1,4 +1,5 @@ fn main
     one
    -two
    +TWO
    +extra
     three
    @@ -20 +21 @@
    -old tail
    +new tail
    diff --git a/docs/new file.md b/docs/new file.md
    new file mode 100644
    --- /dev/null
    +++ b/docs/new file.md
    @@ -0,0 +1,2 @@
    +# Title
    +body
    diff --git a/gone.txt b/gone.txt
    deleted file mode 100644
    --- a/gone.txt
    +++ /dev/null
    @@ -1 +0,0 @@
    -bye
    diff --git a/img.png b/img.png
    Binary files a/img.png and b/img.png differ
    diff --git a/a.txt b/b.txt
    similarity index 90%
    rename from a.txt
    rename to b.txt
    """

    func testParsesFilesAndCounts() {
        let files = UnifiedDiff.parse(patch)
        XCTAssertEqual(files.map(\.path), ["src/main.rs", "docs/new file.md", "gone.txt", "img.png", "b.txt"])
        XCTAssertEqual(files[0].additions, 3)
        XCTAssertEqual(files[0].deletions, 2)
        XCTAssertEqual(files[1].additions, 2)
        XCTAssertEqual(files[2].deletions, 1)
        XCTAssertTrue(files[3].binary)
        XCTAssertEqual(files[4].oldPath, "a.txt")
    }

    func testLineNumbersTrackHunks() {
        let lines = UnifiedDiff.parse(patch)[0].lines
        XCTAssertEqual(lines[0].kind, .hunk)
        let two = lines.first { $0.text == "two" }
        XCTAssertEqual(two?.kind, .remove)
        XCTAssertEqual(two?.oldNumber, 2)
        let big = lines.first { $0.text == "TWO" }
        XCTAssertEqual(big?.newNumber, 2)
        let three = lines.first { $0.text == "three" }
        XCTAssertEqual(three?.oldNumber, 3)
        XCTAssertEqual(three?.newNumber, 4)
        let tail = lines.first { $0.text == "new tail" }
        XCTAssertEqual(tail?.newNumber, 21)
    }

    func testContentLookingLikeHeadersInsideHunkStaysContent() {
        let tricky = """
        diff --git a/x b/x
        --- a/x
        +++ b/x
        @@ -1,2 +1,2 @@
        --- not a header
        +++ also content
        """
        let file = UnifiedDiff.parse(tricky)[0]
        XCTAssertEqual(file.path, "x")
        XCTAssertEqual(file.deletions, 1)
        XCTAssertEqual(file.additions, 1)
    }

    func testHunkRanges() {
        let h = UnifiedDiff.hunkRanges("@@ -10,3 +12 @@ context")
        XCTAssertEqual(h?.oldStart, 10); XCTAssertEqual(h?.oldCount, 3)
        XCTAssertEqual(h?.newStart, 12); XCTAssertEqual(h?.newCount, 1)
        XCTAssertNil(UnifiedDiff.hunkRanges("not a hunk"))
    }

    func testTextDiffFallbackBuildsHunks() {
        let old = (1...10).map { "line \($0)" }.joined(separator: "\n") + "\n"
        let new = old.replacingOccurrences(of: "line 5\n", with: "changed\nadded\n")
        let lines = UnifiedDiff.lines(old: old, new: new, context: 2)
        XCTAssertEqual(lines.filter { $0.kind == .hunk }.count, 1)
        XCTAssertEqual(lines.filter { $0.kind == .remove }.map(\.text), ["line 5"])
        XCTAssertEqual(lines.filter { $0.kind == .add }.map(\.text), ["changed", "added"])
        XCTAssertEqual(lines.first { $0.text == "line 3" }?.oldNumber, 3)
        XCTAssertEqual(lines.first { $0.text == "line 6" }?.newNumber, 7)
        XCTAssertEqual(UnifiedDiff.lines(old: "a\n", new: "a\n"), [])
    }

    func testNewFileTextDiffIsAllAdditions() {
        let lines = UnifiedDiff.lines(old: nil, new: "a\nb\n")
        XCTAssertEqual(lines.filter { $0.kind == .add }.count, 2)
        XCTAssertEqual(lines.first?.text, "@@ -0,0 +1,2 @@")
    }

    func testDiffFileStatusLetters() {
        XCTAssertEqual(HostDiffFile(path: "a", status: "added").letter, "A")
        XCTAssertEqual(HostDiffFile(path: "a", status: "deleted").letter, "D")
        XCTAssertEqual(HostDiffFile(path: "a", status: "renamed").letter, "R")
        XCTAssertEqual(HostDiffFile(path: "a", status: "modified").letter, "M")
    }

    func testCheckoutDiffDecodesHostShape() throws {
        let json = """
        {"checkoutId":"c1","deviceId":"d","cwd":"/r","patch":"","files":[{"path":"a.rs","status":"added","additions":3,"deletions":0}],
         "additions":3,"deletions":0,"truncated":false,"checksum":"abc","updatedAt":"2026-09-30T00:00:00Z"}
        """
        let diff = try JSONDecoder().decode(HostCheckoutDiff.self, from: Data(json.utf8))
        XCTAssertEqual(diff.files.first?.letter, "A")
        XCTAssertFalse(diff.files[0].binary)
        XCTAssertEqual(diff.checksum, "abc")
    }
}

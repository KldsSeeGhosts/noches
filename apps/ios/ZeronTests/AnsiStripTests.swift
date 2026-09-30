import XCTest
@testable import Zeron

final class AnsiStripTests: XCTestCase {
    private func render(_ chunks: String...) -> [String] {
        var buffer = TerminalBuffer()
        for chunk in chunks { buffer.feed(Data(chunk.utf8)) }
        return buffer.plainLines
    }

    func testStripsColorAndPrivateSequences() {
        XCTAssertEqual(render("\u{1B}[31mred\u{1B}[0m ok\u{1B}[?2004h"), ["red ok"])
        XCTAssertEqual(render("\u{1B}]0;title\u{07}hi"), ["hi"])
        XCTAssertEqual(render("\u{1B}]0;title\u{1B}\\hi"), ["hi"])
        XCTAssertEqual(render("\u{1B}(Bplain"), ["plain"])
    }

    func testCarriageReturnOverwritesAndNewlines() {
        XCTAssertEqual(render("loading 10%\rloading 99%\r\ndone\r\n"), ["loading 99%", "done", ""])
        XCTAssertEqual(render("abcdef\rXY"), ["XYcdef"])
    }

    func testBackspaceAndEraseLine() {
        XCTAssertEqual(render("abc\u{08}\u{1B}[K"), ["ab"])
        XCTAssertEqual(render("abc\u{08} \u{08}"), ["ab "])
        XCTAssertEqual(render("hello\u{1B}[2Kx"), ["     x"])
    }

    func testSequenceAndUTF8SplitAcrossChunks() {
        XCTAssertEqual(render("a\u{1B}", "[3", "1mb"), ["ab"])
        let bytes = Array("é".utf8)
        var buffer = TerminalBuffer()
        buffer.feed(Data([bytes[0]]))
        XCTAssertEqual(buffer.plainLines, [""])
        buffer.feed(Data([bytes[1]]))
        XCTAssertEqual(buffer.plainLines, ["é"])
    }

    func testColorsAndTabsAndClear() {
        var buffer = TerminalBuffer()
        buffer.feed("\u{1B}[32mok\u{1B}[0m x")
        XCTAssertEqual(buffer.lines[0][0].color, 2)
        XCTAssertNil(buffer.lines[0].last?.color)
        XCTAssertEqual(render("a\tb"), ["a       b"])
        XCTAssertEqual(render("junk\r\nmore\u{1B}[2Jfresh"), ["fresh"])
    }

    func testScrollbackIsBounded() {
        var buffer = TerminalBuffer(limit: 10)
        for i in 0..<50 { buffer.feed("line \(i)\r\n") }
        XCTAssertLessThanOrEqual(buffer.lines.count, 11)
        XCTAssertGreaterThan(buffer.dropped, 0)
    }

    func testEventDecoding() {
        let data = JSONValue.object(["type": .string("data"), "seq": .int(3), "data": .string("aGk=")])
        XCTAssertEqual(HostTerminalEvent(data), .data(seq: 3, bytes: Data("hi".utf8)))
        let exit = JSONValue.object(["type": .string("exit"), "seq": .int(4), "exitCode": .int(2)])
        XCTAssertEqual(HostTerminalEvent(exit), .exit(seq: 4, code: 2, signal: nil))
    }
}

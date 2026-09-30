import XCTest
@testable import Zeron

final class IncrementalMarkdownTests: XCTestCase {
    func testUnicodeStreamingAndCorrectionsMatchWholeDocumentParsing() {
        let parser = IncrementalMarkdownParser()
        let texts = [
            "# Heading\n\nCafé 👨",
            "# Heading\n\nCafé 👨‍💻\n\nNext",
            "# Heading\n\nCafé 👨‍💻\n\nNext 界\n\n```swift\nlet value = 1",
            "# Heading\n\nCafé 👨‍💻\n\nNext 界\n\n```swift\nlet value = 1\n```\n\nDone",
            "# Corrected\n\nCafé 👨‍💻\n\nA correction",
            "# Corrected\n\nCafé 👨‍💻\n\nA correction\n\n[ref]: https://example.invalid\n\n[link][ref]",
        ]
        for text in texts {
            parser.setText(text)
            XCTAssertEqual(parser.blocks, MarkdownParser.parse(text))
        }
    }

    func testCanonicalEquivalentButByteDifferentPrefixResetsIndexesSafely() {
        let parser = IncrementalMarkdownParser()
        parser.setText("# Header\n\nCafé")
        let text = "# Header\n\nCafe\u{301}\n\nNew block"
        parser.setText(text)
        XCTAssertEqual(parser.blocks, MarkdownParser.parse(text))
    }

    func testLongPrefixUsesIndexedLineBoundaries() {
        let parser = IncrementalMarkdownParser()
        var text = (0..<500).map { "Paragraph \($0)\n\n" }.joined()
        parser.setText(text)
        for value in ["Tail", " 🦊", "\n\n", "More", "\n\n- item\n", "- other"] {
            text += value
            parser.setText(text)
            XCTAssertEqual(parser.blocks, MarkdownParser.parse(text))
        }
    }

    func testCarriageReturnAndSplitCRLFBoundaries() {
        let parser = IncrementalMarkdownParser()
        for text in ["# Heading\r\rFirst\r", "# Heading\r\rFirst\r\n\r\nTail",
                     "# Heading\r\rFirst\r\n\r\nTail\r\n\r\nLast"] {
            parser.setText(text)
            XCTAssertEqual(parser.blocks, MarkdownParser.parse(text))
        }
    }
}

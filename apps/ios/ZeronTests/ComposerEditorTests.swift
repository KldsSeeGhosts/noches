import SwiftUI
import XCTest
@testable import Zeron

@MainActor
final class ComposerEditorTests: XCTestCase {
    func testSendingMarkedTextClearsNativeStorageWithoutLosingFocusOrNextDraft() async throws {
        // XCTest can begin before SwiftUI's initial scene connects. Report a
        // bounded setup failure instead of crashing/relaunching the whole suite.
        for _ in 0..<100 {
            if UIApplication.shared.connectedScenes.contains(where: { $0 is UIWindowScene }) { break }
            try await Task.sleep(for: .milliseconds(50))
        }
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let window = UIWindow(windowScene: scene)
        let host = UIViewController()
        window.rootViewController = host
        let input = UITextView(frame: CGRect(x: 20, y: 100, width: 300, height: 100))
        host.view.addSubview(input)
        window.makeKeyAndVisible()
        defer { window.isHidden = true; window.rootViewController = nil }
        let editor = ComposerEditorController()
        editor.view = input
        input.delegate = editor
        var draft = ""
        editor.textChanged = { draft = $0 }
        input.becomeFirstResponder()
        for prompt in ["hellp", "你好 👋🏽", "First line\nSecond line"] {
            input.setMarkedText(prompt, selectedRange: NSRange(location: prompt.utf16.count, length: 0))
            editor.commit()
            XCTAssertEqual(draft, prompt)
            XCTAssertNil(input.markedTextRange)
            draft = ""
            editor.apply(text: draft)
            XCTAssertEqual(input.text, "")
            XCTAssertTrue(input.isFirstResponder)
            // UIKit may deliver a queued change notification after clearing.
            editor.textViewDidChange(input)
            XCTAssertEqual(draft, "")
            input.insertText("Next draft")
            try? await Task.sleep(for: .milliseconds(100))
            XCTAssertEqual(draft, "Next draft")
            XCTAssertEqual(input.text, "Next draft")
            editor.apply(text: "")
        }
    }
}

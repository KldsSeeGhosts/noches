import Foundation
import Loro
import XCTest
@testable import Zeron

final class PersistenceSafetyTests: XCTestCase {
    func testPruningKeepsOldPendingAndUnreadableSnapshotsOutsideCacheBudget() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        for index in 0..<85 {
            let url = directory.appendingPathComponent("c2_\(index).loro")
            var header = Data("C2SNAP02".utf8)
            header.append(Data(repeating: 0, count: 8))
            header.append(index == 0 ? 14 : 10)
            try header.write(to: url)
            try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: Double(index))],
                                                  ofItemAtPath: url.path)
        }
        let unknown = directory.appendingPathComponent("unreadable.loro")
        try Data("unrecognized snapshot".utf8).write(to: unknown)
        DocDisk.prune(keep: 2, in: directory)
        let files = try FileManager.default.contentsOfDirectory(atPath: directory.path)
        XCTAssertEqual(Set(files), ["c2_0.loro", "c2_83.loro", "c2_84.loro", "unreadable.loro"])
    }

    func testPendingQueueRowsArePinnedIncludingOldClearCommandOnlyHeaders() throws {
        let id = "queue-durability-\(UUID().uuidString)"
        defer { try? FileManager.default.removeItem(at: DocDisk.chat2URL(for: id)) }
        let doc = LoroDoc()
        let row = try doc.getList(id: "queue").pushContainer(child: LoroMap())
        try row.insert(key: "id", v: "unsent")
        try row.insert(key: "text", v: "offline queue message")
        try DocDisk.saveChat2(doc: doc, id: id, cursor: 0, verified: false)
        XCTAssertTrue(DocDisk.hasPendingCommands(id: id))
        var old = try Data(contentsOf: DocDisk.chat2URL(for: id))
        old[16] = 2 // Old metadata says no commands, but did not inspect queue rows.
        try old.write(to: DocDisk.chat2URL(for: id))
        XCTAssertTrue(DocDisk.hasPendingCommands(id: id))
    }

    @MainActor
    func testFailedSaveKeepsDirtyObligationAndClearsErrorOnlyAfterSuccess() {
        var failing = true
        var attempts = 0
        var errorSeen = false
        let saver = DocSaver(onError: { errorSeen = $0 != nil }) {
            attempts += 1
            if failing { throw CocoaError(.fileWriteOutOfSpace) }
        }
        saver.poke()
        XCTAssertFalse(saver.flush())
        XCTAssertTrue(saver.needsSave)
        XCTAssertTrue(errorSeen)
        failing = false
        XCTAssertTrue(saver.flush())
        XCTAssertFalse(saver.needsSave)
        XCTAssertFalse(errorSeen)
        XCTAssertEqual(attempts, 2)
        XCTAssertTrue(saver.flush())
        XCTAssertEqual(attempts, 2)
    }

    @MainActor
    func testRegistryWriteFailureRetriesWithoutAnotherMutation() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("registry.json")
        let bytes = Data("durable outbox".utf8)
        let saver = RegistrySaver(url: url) { bytes }
        saver.poke()
        XCTAssertFalse(saver.flush(), "missing parent must not count as a successful write")
        XCTAssertTrue(saver.needsSave)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        XCTAssertTrue(saver.flush())
        XCTAssertFalse(saver.needsSave)
        XCTAssertEqual(try Data(contentsOf: url), bytes)
    }
}

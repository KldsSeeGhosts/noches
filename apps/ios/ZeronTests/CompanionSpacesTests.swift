import XCTest
@testable import Zeron

final class CompanionSpacesTests: XCTestCase {
    private func decode<T: Decodable>(_ type: T.Type, _ text: String) throws -> T {
        try JSONDecoder().decode(type, from: Data(text.utf8))
    }

    func testFolderListingDecodesRustShape() throws {
        let listing = try decode(HostFolderListing.self,
            #"{"path":"/Users/me","entries":[{"name":"code","isDir":true,"isRepo":false},{"name":"noches","isDir":true,"isRepo":true}],"truncated":true}"#)
        XCTAssertEqual(listing.path, "/Users/me")
        XCTAssertEqual(listing.entries.map(\.name), ["code", "noches"])
        XCTAssertEqual(listing.entries.map(\.isRepo), [false, true])
        XCTAssertTrue(listing.truncated)
        // `truncated` and `entries` are serde defaults on older hosts.
        let bare = try decode(HostFolderListing.self, #"{"path":"/"}"#)
        XCTAssertFalse(bare.truncated)
        XCTAssertTrue(bare.entries.isEmpty)
    }

    func testDrivesAndWorktreeDecode() throws {
        struct Listing: Decodable { var drives: [HostDrive] }
        let drives = try decode(Listing.self, #"{"drives":[{"name":"System","path":"/"},{"name":"Dev","path":"/Volumes/Dev"}]}"#).drives
        XCTAssertEqual(drives.map(\.path), ["/", "/Volumes/Dev"])
        let tree = try decode(HostWorktree.self,
            #"{"repoPath":"/r","path":"/w/r/calm-fox","branch":"zeron/calm-fox","name":"calm-fox","checkoutId":"abc"}"#)
        XCTAssertEqual(tree.branch, "zeron/calm-fox")
        XCTAssertEqual(tree.path, "/w/r/calm-fox")
    }

    func testHostPathHelpers() {
        XCTAssertEqual(CompanionModel.normalizedHostPath(" /Users/me/ "), "/Users/me")
        XCTAssertEqual(CompanionModel.normalizedHostPath("/"), "/")
        XCTAssertEqual(CompanionModel.normalizedHostPath("C:\\"), "C:\\")
        XCTAssertEqual(CompanionModel.parentHostPath("/Users/me"), "/Users")
        XCTAssertEqual(CompanionModel.parentHostPath("/Users"), "/")
        XCTAssertNil(CompanionModel.parentHostPath("/"))
        XCTAssertEqual(CompanionModel.parentHostPath("C:\\Users\\me"), "C:\\Users")
        XCTAssertEqual(CompanionModel.parentHostPath("C:\\Users"), "C:\\")
        XCTAssertNil(CompanionModel.parentHostPath("C:\\"))
    }

    @MainActor func testRequestsNeedAnOnlineHost() async {
        let model = CompanionModel()
        do { _ = try await model.listFolders(path: nil); XCTFail("expected notConnected") } catch {}
        do { _ = try await model.addProject(path: "/tmp", name: nil, isRepo: false); XCTFail("expected notConnected") } catch {}
    }
}
